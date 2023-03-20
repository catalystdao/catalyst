use std::ops::{Div, Sub};

use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128, DepsMut, Env, Response, Event, MessageInfo, Deps, StdResult, CosmosMsg, to_binary, Timestamp, Storage};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::Map;
use cw20_base::{state::{MinterData, TokenInfo, TOKEN_INFO}, contract::execute_mint};
use ethnum::{U256, uint};
use fixed_point_math_lib::fixed_point_math::mul_wad_down;
use sha3::{Digest, Keccak256};

use crate::ContractError;


pub const MAX_ASSETS: usize = 3;

pub const DECIMALS: u8 = 18;
pub const INITIAL_MINT_AMOUNT: Uint128 = Uint128::new(1000000000000000000u128); // 1e18

pub const MAX_POOL_FEE_SHARE       : u64 = 1000000000000000000u64;              // 100%
pub const MAX_GOVERNANCE_FEE_SHARE : u64 = 75u64 * 10000000000000000u64;        // 75%    //TODO EVM mismatch (move to factory)

pub const DECAY_RATE: U256 = uint!("86400");    // 60*60*24

pub const ASSET_ESCROWS: Map<&str, String> = Map::new("catalyst-pool-asset-escrows");
pub const LIQUIDITY_ESCROWS: Map<&str, String> = Map::new("catalyst-pool-liquidity-escrows");
pub const CONNECTIONS: Map<(&str, &str), bool> = Map::new("catalyst-pool-connections");   //TODO channelId and toPool types

// TODO move to utils/similar?
fn calc_keccak256(message: Vec<u8>) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(message);
    format!("{:?}", hasher.finalize().to_vec())
}


pub trait CatalystV1PoolState: Sized {

    // State access functions

    fn new_unsafe() -> Self;

    fn load_state(store: &dyn Storage) -> StdResult<Self>;
    fn save_state(self, store: &mut dyn Storage) -> StdResult<()>;

    fn factory(&self) -> &Addr;
    fn factory_owner(&self) -> &Addr;

    fn setup_master(&self) -> &Option<Addr>;
    fn setup_master_mut(&mut self) -> &mut Option<Addr>;

    fn chain_interface(&self) -> &Option<Addr>;
    fn chain_interface_mut(&mut self) -> &mut Option<Addr>;

    fn assets(&self) -> &Vec<Addr>;
    fn assets_mut(&mut self) -> &mut Vec<Addr>;

    fn weights(&self) -> &Vec<u64>;
    fn weights_mut(&mut self) -> &mut Vec<u64>;

    fn amplification(&self) -> &u64;
    fn amplification_mut(&mut self) -> &mut u64;

    fn fee_administrator(&self) -> &Addr;
    fn fee_administrator_mut(&mut self) -> &mut Addr;

    fn pool_fee(&self) -> &u64;
    fn pool_fee_mut(&mut self) -> &mut u64;

    fn governance_fee(&self) -> &u64;
    fn governance_fee_mut(&mut self) -> &mut u64;

    fn escrowed_assets(&self) -> &Vec<Uint128>;
    fn escrowed_assets_mut(&mut self) -> &mut Vec<Uint128>;

    fn escrowed_pool_tokens(&self) -> &Uint128;
    fn escrowed_pool_tokens_mut(&mut self) -> &mut Uint128;

    fn max_limit_capacity(&self) -> &U256;           // TODO EVM mismatch (name maxUnitCapacity)
    fn max_limit_capacity_mut(&mut self) -> &mut U256;

    fn used_limit_capacity(&self) -> &U256;          // TODO EVM mismatch (name usedUnitCapacity)
    fn used_limit_capacity_mut(&mut self) -> &mut U256;

    fn used_limit_capacity_timestamp(&self) -> &u64;
    fn used_limit_capacity_timestamp_mut(&mut self) -> &mut u64;



    // Default implementations

    fn get_asset_index(&self, asset: &str) -> Result<usize, ContractError> {
        self.assets()
            .iter()
            .enumerate()
            .find_map(|(index, a): (usize, &Addr)| if *a == asset { Some(index) } else { None })
            .ok_or(ContractError::InvalidAssets {})
    }


    fn only_local(deps: Deps) -> StdResult<bool> {
    
        let state = Self::load_state(deps.storage)?;

        Ok(state.chain_interface().is_none())
    }


    fn ready(deps: Deps) -> StdResult<bool> {
    
        let state = Self::load_state(deps.storage)?;

        Ok(state.setup_master().is_none() && state.assets().len() > 0)
    }

    
    //TODO move these somewhere else? (Note both update_unit_capacity and get_unit_capacity (in Derived) depend on calc_unit_capacity)
    fn calc_unit_capacity(
        &self,
        time: Timestamp
    ) -> Result<U256, ContractError> {

        let max_limit_capacity = *self.max_limit_capacity();
        let used_limit_capacity = *self.used_limit_capacity();

        let released_limit_capacity = max_limit_capacity
            .checked_mul(
                U256::from(time.minus_nanos(*self.used_limit_capacity_timestamp()).seconds())  //TODO use seconds instead of nanos (overflow wise)
            ).ok_or(ContractError::ArithmeticError {})?   //TODO error
            .div(DECAY_RATE);

            if used_limit_capacity <= released_limit_capacity {
                return Ok(max_limit_capacity);
            }

            if max_limit_capacity <= used_limit_capacity - released_limit_capacity {
                return Ok(U256::ZERO);
            }

            Ok(
                max_limit_capacity
                    .checked_add(released_limit_capacity).ok_or(ContractError::ArithmeticError {})?
                    .sub(used_limit_capacity)
            )

    }

    fn update_unit_capacity(
        deps: &mut DepsMut,
        env: Env,
        units: U256
    ) -> Result<(), ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        //TODO EVM mismatch
        let capacity = state.calc_unit_capacity(env.block.time)?;

        if units > capacity {
            return Err(ContractError::SecurityLimitExceeded { units, capacity });
        }

        let used_limit_capacity_timestamp = state.used_limit_capacity_timestamp_mut();
        *used_limit_capacity_timestamp = env.block.time.nanos();

        let used_limit_capacity = state.used_limit_capacity_mut();
        *used_limit_capacity = capacity - units;

        state.save_state(deps.storage)?;

        Ok(())
    }


}    



pub trait CatalystV1PoolAdministration: CatalystV1PoolState {

    //TODO provide a basic default implementation?
    fn initialize_swap_curves(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        assets: Vec<String>,
        assets_balances: Vec<Uint128>,  //TODO EVM MISMATCH
        weights: Vec<u64>,
        amp: u64,
        depositor: String
    ) -> Result<Response, ContractError>;


    fn finish_setup(
        deps: &mut DepsMut,
        info: MessageInfo
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        let setup_master = state.setup_master_mut();

        if *setup_master != Some(info.sender) {
            return Err(ContractError::Unauthorized {})
        }

        *setup_master = None;
        state.save_state(deps.storage)?;

        Ok(Response::new())
    }

    
    fn set_connection(
        deps: &mut DepsMut,
        info: MessageInfo,
        channel_id: String,
        to_pool: String,
        state: bool
    ) -> Result<Response, ContractError> {
        let pool_state = Self::load_state(deps.storage)?;

        if *pool_state.setup_master() != Some(info.sender) {   // TODO check also for factory owner
            return Err(ContractError::Unauthorized {});
        }

        CONNECTIONS.save(deps.storage, (channel_id.as_str(), to_pool.as_str()), &state)?;

        Ok(
            Response::new()
                .add_attribute("channel_id", channel_id)
                .add_attribute("to_pool", to_pool)
                .add_attribute("state", state.to_string())
        )
    }



    fn set_fee_administrator_unchecked(
        &mut self,
        deps: &DepsMut,
        administrator: &str
    ) -> Result<Event, ContractError> {

        let fee_administrator = self.fee_administrator_mut();
        *fee_administrator = deps.api.addr_validate(administrator)?;

        return Ok(
            Event::new(String::from("SetFeeAdministrator"))
                .add_attribute("administrator", administrator)
        )
    }

    fn set_fee_administrator(
        deps: &mut DepsMut,
        info: MessageInfo,
        administrator: String
    ) -> Result<Response, ContractError> {
        let mut state = Self::load_state(deps.storage)?;

        //TODO verify sender is factory owner

        let event = state.set_fee_administrator_unchecked(deps, administrator.as_str())?;

        state.save_state(deps.storage)?;

        Ok(Response::new().add_event(event))
    }



    fn set_pool_fee_unchecked(
        &mut self,
        fee: u64
    ) -> Result<Event, ContractError> {

        if fee > MAX_POOL_FEE_SHARE {
            return Err(
                ContractError::InvalidPoolFee { requested_fee: fee, max_fee: MAX_POOL_FEE_SHARE }
            )
        }

        let pool_fee = self.pool_fee_mut();
        *pool_fee = fee;

        return Ok(
            Event::new(String::from("SetPoolFee"))
                .add_attribute("fee", fee.to_string())
        )
    }

    fn set_pool_fee(
        deps: &mut DepsMut,
        info: MessageInfo,
        fee: u64
    ) -> Result<Response, ContractError> {
        let mut state = Self::load_state(deps.storage)?;

        if info.sender != *state.fee_administrator() {
            return Err(ContractError::Unauthorized {})
        }

        let event = state.set_pool_fee_unchecked(fee)?;

        state.save_state(deps.storage)?;

        Ok(Response::new().add_event(event))
    }



    fn set_governance_fee_unchecked(
        &mut self,
        fee: u64
    ) -> Result<Event, ContractError> {

        if fee > MAX_GOVERNANCE_FEE_SHARE {
            return Err(
                ContractError::InvalidGovernanceFee { requested_fee: fee, max_fee: MAX_GOVERNANCE_FEE_SHARE }
            )
        }

        let governance_fee = self.governance_fee_mut();
        *governance_fee = fee;

        return Ok(
            Event::new(String::from("SetGovernanceFee"))
                .add_attribute("fee", fee.to_string())
        )
    }

    fn set_governance_fee(
        deps: &mut DepsMut,
        info: MessageInfo,
        fee: u64
    ) -> Result<Response, ContractError> {
        let mut state = Self::load_state(deps.storage)?;

        if info.sender != *state.fee_administrator() {
            return Err(ContractError::Unauthorized {})
        }

        let event = state.set_governance_fee_unchecked(fee)?;

        state.save_state(deps.storage)?;

        Ok(Response::new().add_event(event))
    }

    fn collect_governance_fee_message(
        &self,
        env: Env,
        asset: String,
        pool_fee_amount: Uint128
    ) -> Result<CosmosMsg, ContractError> {

        let gov_fee_amount: Uint128 = mul_wad_down(
            U256::from(pool_fee_amount.u128()),
            U256::from(self.governance_fee().clone())
        )?.as_u128().into();     //TODO unsafe as_u128 casting

        Ok(CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: asset,
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.contract.address.to_string(),
                    recipient: self.factory_owner().to_string(),
                    amount: gov_fee_amount
                })?,
                funds: vec![]
            }
        ))
        
    }

}



pub trait CatalystV1PoolPermissionless: CatalystV1PoolState + CatalystV1PoolAdministration {

    //TODO merge setup and initializeSwapCurves?
    fn setup(
        deps: &mut DepsMut,
        env: &Env,
        name: String,
        symbol: String,
        chain_interface: Option<String>,
        pool_fee: u64,
        governance_fee: u64,
        fee_administrator: String,
        setup_master: String,
    ) -> Result<Response, ContractError> {

        let mut state = Self::new_unsafe();

        let setup_master_state = state.setup_master_mut();
        *setup_master_state = Some(deps.api.addr_validate(&setup_master)?);
    
        let chain_interface_state = state.chain_interface_mut();
        *chain_interface_state = match chain_interface {
            Some(chain_interface) => Some(deps.api.addr_validate(&chain_interface)?),
            None => None
        };


        let admin_fee_event = state.set_fee_administrator_unchecked(deps, fee_administrator.as_str())?;
        let pool_fee_event = state.set_pool_fee_unchecked(pool_fee)?;
        let gov_fee_event = state.set_governance_fee_unchecked(governance_fee)?;

        state.save_state(deps.storage)?;

        // Setup the Pool Token (store token info using cw20-base format)
        let data = TokenInfo {
            name,
            symbol,
            decimals: DECIMALS,
            total_supply: Uint128::zero(),
            mint: Some(MinterData {
                minter: env.contract.address.clone(),  // Set self as minter
                cap: None
            })
        };
        TOKEN_INFO.save(deps.storage, &data)?;

        Ok(
            Response::new()
                .add_event(admin_fee_event)
                .add_event(pool_fee_event)
                .add_event(gov_fee_event)
        )
    }

    //TODO depositMixed

    //TODO withdrawAll

    //TODO withdrawMixed

    fn local_swap(
        deps: &Deps,
        env: Env,
        info: MessageInfo,
        from_asset: String,
        to_asset: String,
        amount: Uint128,
        min_out: Uint128
    ) -> Result<Response, ContractError>;

    //TODO sendAsset

    //TODO receiveAsset

    //TODO sendLiquidity

    //TODO receiveLiquidity

}



pub trait CatalystV1PoolDerived: CatalystV1PoolState {

    //TODO depend directly on self
    fn get_unit_capacity(
        deps: Deps,
        env: Env
    ) -> Result<U256, ContractError> {

        let state = Self::load_state(deps.storage)?;

        state.calc_unit_capacity(env.block.time)
    }

    fn calc_send_asset(
        &self,
        deps: Deps,
        env: Env,
        from_asset: &str,
        amount: Uint128
    ) -> Result<U256, ContractError>;

    fn calc_receive_asset(
        &self,
        deps: Deps,
        env: Env,
        to_asset: &str,
        u: U256
    ) -> Result<Uint128, ContractError>;

    fn calc_local_swap(
        &self,
        deps: Deps,
        env: Env,
        from_asset: &str,
        to_asset: &str,
        amount: Uint128
    ) -> Result<Uint128, ContractError>;

}



pub trait CatalystV1PoolAckTimeout: CatalystV1PoolState + CatalystV1PoolAdministration {

    fn release_asset_escrow(
        &mut self,
        deps: &mut DepsMut,
        send_asset_hash: &str,
        amount: Uint128,
        asset: &str
    ) -> Result<String, ContractError> {

        let fallback_account = ASSET_ESCROWS.load(deps.storage, send_asset_hash)?;
        ASSET_ESCROWS.remove(deps.storage, send_asset_hash);

        let asset_index = self.get_asset_index(asset)?;
        let escrowed_assets = self.escrowed_assets_mut();
        escrowed_assets[asset_index] -= amount;               // Safe, as 'amount' is always contained in 'escrowed_assets'

        Ok(fallback_account)
    }


    fn release_liquidity_escrow(
        &mut self,
        deps: &mut DepsMut,
        send_liquidity_hash: &str,
        amount: Uint128
    ) -> Result<String, ContractError> {

        let fallback_account = LIQUIDITY_ESCROWS.load(deps.storage, send_liquidity_hash)?;
        LIQUIDITY_ESCROWS.remove(deps.storage, send_liquidity_hash);

        let escrowed_pool_tokens = self.escrowed_pool_tokens_mut();
        *escrowed_pool_tokens -= amount;               // Safe, as 'amount' is always contained in 'escrowed_assets'

        Ok(fallback_account)
    }



    fn on_send_asset_ack(
        &mut self,
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        if Some(info.sender) != *self.chain_interface() {
            return Err(ContractError::Unauthorized {})
        }

        let send_asset_hash = Self::compute_send_asset_hash(
            to_account.as_str(),
            u,
            amount,
            asset.as_str(),
            block_number_mod
        );

        self.release_asset_escrow(deps, &send_asset_hash, amount, &asset)?;

        Ok(
            Response::new()
                .add_attribute("swap_hash", send_asset_hash)
        )
    }

    fn send_asset_ack(
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        let response = state.on_send_asset_ack(
            deps,
            info,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        )?;

        state.save_state(deps.storage)?;

        Ok(response)
    }



    fn on_send_asset_timeout(
        &mut self,
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        if Some(info.sender) != *self.chain_interface() {
            return Err(ContractError::Unauthorized {})
        }

        let send_asset_hash = Self::compute_send_asset_hash(
            to_account.as_str(),
            u,
            amount,
            asset.as_str(),
            block_number_mod
        );

        let fallback_address = self.release_asset_escrow(deps, &send_asset_hash, amount, &asset)?;

        // Transfer escrowed asset to fallback user
        let transfer_msg: CosmosMsg = CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: asset.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.contract.address.to_string(),
                    recipient: fallback_address,
                    amount
                })?,
                funds: vec![]
            }
        );

        Ok(
            Response::new()
                .add_message(transfer_msg)
                .add_attribute("swap_hash", send_asset_hash)
        )
    }

    fn send_asset_timeout(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        let response = state.on_send_asset_timeout(
            deps,
            env,
            info,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        )?;

        state.save_state(deps.storage)?;

        Ok(response)
    }



    fn on_send_liquidity_ack(
        &mut self,
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        if Some(info.sender) != *self.chain_interface() {
            return Err(ContractError::Unauthorized {})
        }

        let send_liquidity_hash = Self::compute_send_liquidity_hash(
            to_account.as_str(),
            u,
            amount,
            block_number_mod
        );

        self.release_liquidity_escrow(deps, &send_liquidity_hash, amount)?;

        Ok(
            Response::new()
                .add_attribute("swap_hash", send_liquidity_hash)
        )
    }

    fn send_liquidity_ack(
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        let response = state.on_send_liquidity_ack(
            deps,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        )?;

        state.save_state(deps.storage)?;

        Ok(response)
    }



    fn on_send_liquidity_timeout(
        &mut self,
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        if Some(info.sender) != *self.chain_interface() {
            return Err(ContractError::Unauthorized {})
        }

        let send_liquidity_hash = Self::compute_send_liquidity_hash(
            to_account.as_str(),
            u,
            amount,
            block_number_mod
        );

        let fallback_address = self.release_liquidity_escrow(deps, &send_liquidity_hash, amount)?;

        // Mint pool tokens for the fallbackAccount
        let execute_mint_info = MessageInfo {
            sender: env.contract.address.clone(),
            funds: vec![],
        };
        let mint_response = execute_mint(
            deps.branch(),  //TODO is '.branch()' correct to get a copy of the object?
            env,
            execute_mint_info,
            fallback_address,
            amount
        )?;

        Ok(
            Response::new()
                .add_attribute("swap_hash", send_liquidity_hash)
                .add_attributes(mint_response.attributes)   //TODO better way to do this?
        )
    }

    fn send_liquidity_timeout(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        let response = state.on_send_liquidity_timeout(
            deps,
            env,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        )?;

        state.save_state(deps.storage)?;

        Ok(response)
    }



    fn compute_send_asset_hash(
        to_account: &str,
        u: U256,
        amount: Uint128,
        asset: &str,
        block_number_mod: u32        
    ) -> String {
        
        let to_account_bytes = to_account.as_bytes();
        let asset_bytes = asset.as_bytes();

        let mut hash_data: Vec<u8> = Vec::with_capacity(    // Initialize vec with the specified capacity (avoid reallocations)
            to_account_bytes.len()
                + 32
                + 16
                + asset_bytes.len()
                + 4
        );

        hash_data.extend_from_slice(to_account_bytes);
        hash_data.extend_from_slice(&u.to_be_bytes());
        hash_data.extend_from_slice(&amount.to_be_bytes());
        hash_data.extend_from_slice(asset_bytes);
        hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
        
        calc_keccak256(hash_data)
    }

    fn compute_send_liquidity_hash(
        to_account: &str,
        u: U256,
        amount: Uint128,
        block_number_mod: u32        
    ) -> String {
        
        let to_account_bytes = to_account.as_bytes();

        let mut hash_data: Vec<u8> = Vec::with_capacity(    // Initialize vec with the specified capacity (avoid reallocations)
            to_account_bytes.len()
                + 32
                + 16
                + 4
        );

        hash_data.extend_from_slice(to_account_bytes);
        hash_data.extend_from_slice(&u.to_be_bytes());
        hash_data.extend_from_slice(&amount.to_be_bytes());
        hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
        
        calc_keccak256(hash_data)
    }

}



#[cw_serde]
pub struct Escrow {
    pub fallback_address: Addr
}
