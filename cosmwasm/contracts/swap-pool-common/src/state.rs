use std::ops::{Div, Sub};

use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128, DepsMut, Env, Response, Event, MessageInfo, Deps, StdResult, CosmosMsg, to_binary};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::{Item, Map};
use cw20_base::{state::{MinterData, TokenInfo, TOKEN_INFO}, contract::execute_mint};
use sha3::{Digest, Keccak256};
use fixed_point_math_lib::{u256::U256};

use crate::ContractError;


pub const MAX_ASSETS: usize = 3;

pub const DECIMALS: u8 = 18;
pub const INITIAL_MINT_AMOUNT: Uint128 = Uint128::new(1000000000000000000u128); // 1e18

pub const MAX_POOL_FEE_SHARE       : u64 = 1000000000000000000u64;              // 100%
pub const MAX_GOVERNANCE_FEE_SHARE : u64 = 75u64 * 10000000000000000u64;        // 75%    //TODO EVM mismatch (move to factory)

pub const DECAY_RATE: u64 = 60*60*24;

pub const STATE: Item<SwapPoolState> = Item::new("catalyst-pool-state");
pub const ASSET_ESCROWS: Map<&str, String> = Map::new("catalyst-pool-asset-escrows");
pub const LIQUIDITY_ESCROWS: Map<&str, String> = Map::new("catalyst-pool-liquidity-escrows");
pub const CONNECTIONS: Map<(&str, &str), bool> = Map::new("catalyst-pool-connections");   //TODO channelId and toPool types

// TODO move to utils/similar?
fn calc_keccak256(message: Vec<u8>) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(message);
    format!("{:?}", hasher.finalize().to_vec())
}

#[cw_serde]
pub struct SwapPoolState {
    pub setup_master: Option<Addr>,
    pub chain_interface: Option<Addr>,

    pub assets: Vec<Addr>,
    pub weights: Vec<u64>,
    pub amplification: u64,
    
    pub fee_administrator: Addr,
    pub pool_fee: u64,
    pub governance_fee: u64,

    pub escrowed_assets: Vec<Uint128>,
    pub escrowed_pool_tokens: Uint128,

    pub max_limit_capacity: [u64; 4],       // TODO EVM mismatch (name maxUnitCapacity) // TODO use U256 directly
    pub used_limit_capacity: [u64; 4],      // TODO EVM mismatch (name maxUnitCapacity) // TODO use U256 directly
    pub used_limit_capacity_timestamp: u64
}


impl SwapPoolState {

    pub fn get_asset_index(&self, asset: &str) -> Result<usize, ContractError> {
        self.assets
            .iter()
            .enumerate()
            .find_map(|(index, a): (usize, &Addr)| if *a == asset { Some(index) } else { None })
            .ok_or(ContractError::InvalidAssets {})
    }


    fn _set_fee_administrator(
        &mut self,
        deps: &DepsMut,
        administrator: &str
    ) -> Result<Event, ContractError> {
        self.fee_administrator = deps.api.addr_validate(administrator)?;

        return Ok(
            Event::new(String::from("SetFeeAdministrator"))
                .add_attribute("administrator", administrator)
        )
    }

    fn _set_pool_fee(
        &mut self,
        fee: u64
    ) -> Result<Event, ContractError> {

        if fee > MAX_POOL_FEE_SHARE {
            return Err(
                ContractError::InvalidPoolFee { requested_fee: fee, max_fee: MAX_POOL_FEE_SHARE }
            )
        }

        self.pool_fee = fee;

        return Ok(
            Event::new(String::from("SetPoolFee"))
                .add_attribute("fee", fee.to_string())
        )
    }

    fn _set_governance_fee(
        &mut self,
        fee: u64
    ) -> Result<Event, ContractError> {

        if fee > MAX_GOVERNANCE_FEE_SHARE {
            return Err(
                ContractError::InvalidGovernanceFee { requested_fee: fee, max_fee: MAX_GOVERNANCE_FEE_SHARE }
            )
        }

        self.pool_fee = fee;

        return Ok(
            Event::new(String::from("SetGovernanceFee"))
                .add_attribute("fee", fee.to_string())
        )
    }

    pub fn set_fee_administrator(
        deps: &mut DepsMut,
        info: MessageInfo,
        administrator: String
    ) -> Result<Response, ContractError> {
        let mut state = STATE.load(deps.storage)?;

        //TODO verify sender is factory owner

        let event = state._set_fee_administrator(deps, administrator.as_str())?;

        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_event(event))
    }

    pub fn set_pool_fee(
        deps: &mut DepsMut,
        info: MessageInfo,
        fee: u64
    ) -> Result<Response, ContractError> {
        let mut state = STATE.load(deps.storage)?;

        if info.sender != state.fee_administrator {
            return Err(ContractError::Unauthorized {})
        }

        let event = state._set_pool_fee(fee)?;

        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_event(event))
    }

    pub fn set_governance_fee(
        deps: &mut DepsMut,
        info: MessageInfo,
        fee: u64
    ) -> Result<Response, ContractError> {
        let mut state = STATE.load(deps.storage)?;

        if info.sender != state.fee_administrator {
            return Err(ContractError::Unauthorized {})
        }

        let event = state._set_governance_fee(fee)?;

        STATE.save(deps.storage, &state)?;

        Ok(Response::new().add_event(event))
    }

    
    pub fn set_connection(
        deps: &mut DepsMut,
        info: MessageInfo,
        channel_id: String,
        to_pool: String,
        state: bool
    ) -> Result<Response, ContractError> {
        let pool_state = STATE.load(deps.storage)?;

        if pool_state.setup_master != Some(info.sender) {   // TODO check also for factory owner
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


    pub fn ready(deps: Deps) -> StdResult<bool> {
    
        let state = STATE.load(deps.storage)?;

        Ok(state.setup_master.is_none() && state.assets.len() > 0)
    }


    pub fn only_local(deps: Deps) -> StdResult<bool> {

        let state = STATE.load(deps.storage)?;

        Ok(state.chain_interface.is_none())
    }


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
        self.escrowed_assets[asset_index] -= amount;               // Safe, as 'amount' is always contained in 'escrowed_assets'

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

        self.escrowed_pool_tokens -= amount;               // Safe, as 'amount' is always contained in 'escrowed_assets'

        Ok(fallback_account)
    }

    pub fn send_asset_ack(
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,    // TODO maths
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = STATE.load(deps.storage)?;

        if Some(info.sender) != state.chain_interface {
            return Err(ContractError::Unauthorized {})
        }

        let send_asset_hash = SwapPoolState::compute_send_asset_hash(
            to_account.as_str(),
            u,
            amount,
            asset.as_str(),
            block_number_mod
        );

        state.release_asset_escrow(deps, &send_asset_hash, amount, &asset)?;

        STATE.save(deps.storage, &state)?;

        Ok(
            Response::new()
                .add_attribute("swap_hash", send_asset_hash)
        )
    }

    pub fn send_asset_timeout(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        to_account: String,
        u: U256,    // TODO maths
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = STATE.load(deps.storage)?;

        if Some(info.sender) != state.chain_interface {
            return Err(ContractError::Unauthorized {})
        }

        let send_asset_hash = SwapPoolState::compute_send_asset_hash(
            to_account.as_str(),
            u,
            amount,
            asset.as_str(),
            block_number_mod
        );

        let fallback_address = state.release_asset_escrow(deps, &send_asset_hash, amount, &asset)?;

        STATE.save(deps.storage, &state)?;

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

    pub fn send_liquidity_ack(
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,    // TODO maths
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = STATE.load(deps.storage)?;

        if Some(info.sender) != state.chain_interface {
            return Err(ContractError::Unauthorized {})
        }

        let send_liquidity_hash = SwapPoolState::compute_send_liquidity_hash(
            to_account.as_str(),
            u,
            amount,
            block_number_mod
        );

        state.release_liquidity_escrow(deps, &send_liquidity_hash, amount)?;

        STATE.save(deps.storage, &state)?;

        Ok(
            Response::new()
                .add_attribute("swap_hash", send_liquidity_hash)
        )
    }

    pub fn send_liquidity_timeout(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        to_account: String,
        u: U256,    // TODO maths
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, ContractError> {

        let mut state = STATE.load(deps.storage)?;

        if Some(info.sender) != state.chain_interface {
            return Err(ContractError::Unauthorized {})
        }

        let send_liquidity_hash = SwapPoolState::compute_send_liquidity_hash(
            to_account.as_str(),
            u,
            amount,
            block_number_mod
        );

        let fallback_address = state.release_liquidity_escrow(deps, &send_liquidity_hash, amount)?;

        STATE.save(deps.storage, &state)?;

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

    fn compute_send_asset_hash(
        to_account: &str,
        u: U256,                    // TODO maths
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

        hash_data.extend_from_slice(&[0u8; 32usize]);
        u.to_big_endian(&mut hash_data[to_account_bytes.len()..to_account_bytes.len()+32usize]);    

        hash_data.extend_from_slice(&amount.to_be_bytes());
        hash_data.extend_from_slice(asset_bytes);
        hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
        
        calc_keccak256(hash_data)
    }

    fn compute_send_liquidity_hash(
        to_account: &str,
        u: U256,                    // TODO maths
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

        hash_data.extend_from_slice(&[0u8; 32usize]);
        u.to_big_endian(&mut hash_data[to_account_bytes.len()..to_account_bytes.len()+32usize]);    

        hash_data.extend_from_slice(&amount.to_be_bytes());
        hash_data.extend_from_slice(&block_number_mod.to_be_bytes());
        
        calc_keccak256(hash_data)
    }



    pub fn setup(
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

        let setup_master = Some(deps.api.addr_validate(&setup_master)?);
    
        let chain_interface = match chain_interface {
            Some(chain_interface) => Some(deps.api.addr_validate(&chain_interface)?),
            None => None
        };

        let mut state = SwapPoolState {
            setup_master,
            chain_interface,
    
            assets: vec![],
            weights: vec![],
            amplification: 0,
    
            fee_administrator: Addr::unchecked(""),
            pool_fee: 0u64,
            governance_fee: 0u64,
    
            escrowed_assets: vec![],
            escrowed_pool_tokens: Uint128::zero(),
    
            max_limit_capacity: [0u64; 4],
            used_limit_capacity: [0u64; 4],
            used_limit_capacity_timestamp: 0u64
        };

        let admin_fee_event = state._set_fee_administrator(deps, fee_administrator.as_str())?;
        let pool_fee_event = state._set_pool_fee(pool_fee)?;
        let gov_fee_event = state._set_governance_fee(governance_fee)?;

        STATE.save(deps.storage, &state)?;

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

    pub fn finish_setup(
        deps: &mut DepsMut,
        info: MessageInfo
    ) -> Result<Response, ContractError> {
        let mut state = STATE.load(deps.storage)?;

        if state.setup_master != Some(info.sender) {
            return Err(ContractError::Unauthorized {})
        }

        state.setup_master = None;
        STATE.save(deps.storage, &state)?;

        Ok(Response::new())
    }

    pub fn get_unit_capacity(
        deps: Deps,
        env: Env
    ) -> Result<U256, ContractError> {

        let state = STATE.load(deps.storage)?;

        let max_limit_capacity = U256(state.max_limit_capacity);
        let used_limit_capacity = U256(state.used_limit_capacity);

        let released_limit_capacity = max_limit_capacity
            .checked_mul(
                U256::from(env.block.time.minus_nanos(state.used_limit_capacity_timestamp).seconds())
            ).ok_or(ContractError::ArithmeticError {})?   //TODO error
            .div(DECAY_RATE);

            if used_limit_capacity <= released_limit_capacity {
                return Ok(max_limit_capacity);
            }

            if max_limit_capacity <= used_limit_capacity - released_limit_capacity {
                return Ok(U256::zero());
            }

            Ok(
                max_limit_capacity
                    .checked_add(released_limit_capacity).ok_or(ContractError::ArithmeticError {})?
                    .sub(used_limit_capacity)
            )
    }

    // pub fn update_units_inflow(
    //     &mut self,
    //     units_inflow_x64: U256,
    //     current_timestamp: u64
    // ) -> Result<(), ContractError> {

    //     let max_units_inflow_x64 = U256(self.max_units_inflow_x64);

    //     // If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
    //     if current_timestamp > self.current_units_inflow_timestamp + DECAYRATE {
    //         if units_inflow_x64 > max_units_inflow_x64 {
    //             return Err(ContractError::SwapLimitExceeded {});
    //         }

    //         self.current_units_inflow_x64       = units_inflow_x64.0;
    //         self.current_units_inflow_timestamp = current_timestamp;

    //         return Ok(());
    //     }

    //     // Compute how much inflow has decayed since last update
    //     let current_units_inflow_x64 = U256(self.current_units_inflow_x64);

    //     let decayed_inflow = max_units_inflow_x64.checked_mul(
    //         U256::from(current_timestamp.checked_sub(self.current_units_inflow_timestamp).unwrap())  // TODO checked_sub required?
    //     ).unwrap() / DECAYRATE;

    //     // If the current inflow is less then the (max allowed) decayed one
    //     if current_units_inflow_x64 <= decayed_inflow {
    //         if units_inflow_x64 > max_units_inflow_x64 {
    //             return Err(ContractError::SwapLimitExceeded {});
    //         }

    //         self.current_units_inflow_x64 = units_inflow_x64.0;
    //     }
    //     // If some of the current inflow still matters
    //     else {
    //         let new_net_units_inflow_x64 = (current_units_inflow_x64 - decayed_inflow).checked_add(units_inflow_x64).unwrap();  // Substraction is safe, as current_units_inflow_x64 > decayed_inflow is guaranteed by if statement

    //         if new_net_units_inflow_x64 > max_units_inflow_x64 {
    //             return Err(ContractError::SwapLimitExceeded {});
    //         }

    //         self.current_units_inflow_x64 = new_net_units_inflow_x64.0;
    //     }

    //     self.current_units_inflow_timestamp = current_timestamp;

    //     Ok(())
    // }


    // pub fn update_liquidity_units_inflow(
    //     &mut self,
    //     pool_tokens_flow: Uint128,
    //     current_pool_token_supply: Uint128,
    //     current_timestamp: u64
    // ) -> Result<(), ContractError> {

    //     // Allows 1/3 of the pool to be drained through liquidity swaps
    //     let max_pool_tokens_flow = current_pool_token_supply / Uint128::from(2_u64);

    //     // If more time has passed since the last update than DECAYRATE, the current inflow state does not matter (it has fully decayed)
    //     if current_timestamp > self.current_liquidity_inflow_timestamp + DECAYRATE {
    //         if pool_tokens_flow > max_pool_tokens_flow {
    //             return Err(ContractError::LiquiditySwapLimitExceeded {});
    //         }

    //         self.current_liquidity_inflow           = pool_tokens_flow;
    //         self.current_liquidity_inflow_timestamp = current_timestamp;

    //         return Ok(());
    //     }

    //     // Compute how much inflow has decayed since last update
    //     let decayed_inflow = max_pool_tokens_flow.checked_mul(
    //         current_timestamp.checked_sub(self.current_liquidity_inflow_timestamp).unwrap().try_into().unwrap()  // TODO checked_sub required?
    //     ).unwrap() / Uint128::new(DECAYRATE as u128);

    //     // If the current inflow is less then the (max allowed) decayed one
    //     if self.current_liquidity_inflow <= decayed_inflow {
    //         if pool_tokens_flow > max_pool_tokens_flow {
    //             return Err(ContractError::LiquiditySwapLimitExceeded {});
    //         }

    //         self.current_liquidity_inflow = pool_tokens_flow;
    //     }
    //     // If some of the current inflow still matters
    //     else {
    //         let new_net_liquidity_inflow = (self.current_liquidity_inflow - decayed_inflow).checked_add(pool_tokens_flow).unwrap();  // Substraction is safe, as current_liquidity_inflow > decayed_inflow is guaranteed by if statement

    //         if new_net_liquidity_inflow > max_pool_tokens_flow {
    //             return Err(ContractError::LiquiditySwapLimitExceeded {});
    //         }

    //         self.current_liquidity_inflow = new_net_liquidity_inflow;
    //     }

    //     self.current_liquidity_inflow_timestamp = current_timestamp;

    //     Ok(())
    // }

}

#[cw_serde]
pub struct Escrow {
    pub fallback_address: Addr
}
