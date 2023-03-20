use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg, to_binary, Deps};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use cw20_base::contract::execute_mint;
use cw_storage_plus::Item;
use ethnum::U256;
use fixed_point_math_lib::fixed_point_math::LN2;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use swap_pool_common::state::{MAX_ASSETS, INITIAL_MINT_AMOUNT, CatalystV1PoolState, CatalystV1PoolAdministration, CatalystV1PoolAckTimeout, CatalystV1PoolPermissionless, CatalystV1PoolDerived};

use crate::calculation_helpers::{calc_price_curve_area, calc_price_curve_limit, calc_combined_price_curves};

pub const STATE: Item<SwapPoolVolatileState> = Item::new("catalyst-pool-state");

// Implement JsonSchema for U256, see https://graham.cool/schemars/examples/5-remote_derive/
//TODO VERIFY THIS IS CORRECT AND SAFE!
//TODO move to common place
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(remote = "U256")]
pub struct U256Def([u128; 2]);

//TODO rename remove State
#[cw_serde]
pub struct SwapPoolVolatileState {
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

    #[serde(with = "U256Def")]
    pub max_limit_capacity: U256,       // TODO EVM mismatch (name maxUnitCapacity) // TODO use U256 directly
    #[serde(with = "U256Def")]
    pub used_limit_capacity: U256,      // TODO EVM mismatch (name maxUnitCapacity) // TODO use U256 directly
    pub used_limit_capacity_timestamp: u64
}

impl CatalystV1PoolState for SwapPoolVolatileState {

    fn new_unsafe() -> Self {
        SwapPoolVolatileState {
            setup_master: None,
            chain_interface: None,
    
            assets: vec![],
            weights: vec![],
            amplification: 0,
    
            fee_administrator: Addr::unchecked(""),
            pool_fee: 0u64,
            governance_fee: 0u64,
    
            escrowed_assets: vec![],
            escrowed_pool_tokens: Uint128::zero(),
    
            max_limit_capacity: U256::ZERO,
            used_limit_capacity: U256::ZERO,
            used_limit_capacity_timestamp: 0u64
        }
    }


    fn load_state(store: &dyn cosmwasm_std::Storage) -> cosmwasm_std::StdResult<Self> {
        STATE.load(store)
    }

    fn save_state(self, store: &mut dyn cosmwasm_std::Storage) -> cosmwasm_std::StdResult<()> {
        STATE.save(store, &self)
    }


    fn factory(&self) -> &Addr {
        todo!()
    }

    fn factory_owner(&self) -> &Addr {
        todo!()
    }


    fn setup_master(&self) -> &Option<Addr> {
        &self.setup_master
    }

    fn setup_master_mut(&mut self) -> &mut Option<Addr> {
        &mut self.setup_master
    }


    fn chain_interface(&self) -> &Option<Addr> {
        &self.chain_interface
    }

    fn chain_interface_mut(&mut self) -> &mut Option<Addr> {
        &mut self.chain_interface
    }


    fn assets(&self) -> &Vec<Addr> {
        &self.assets
    }

    fn assets_mut(&mut self) -> &mut Vec<Addr> {
        &mut self.assets
    }


    fn weights(&self) -> &Vec<u64> {
        &self.weights
    }

    fn weights_mut(&mut self) -> &mut Vec<u64> {
        &mut self.weights
    }


    fn amplification(&self) -> &u64 {
        &self.amplification
    }

    fn amplification_mut(&mut self) -> &mut u64 {
        &mut self.amplification
    }


    fn fee_administrator(&self) -> &Addr {
        &self.fee_administrator
    }

    fn fee_administrator_mut(&mut self) -> &mut Addr {
        &mut self.fee_administrator
    }


    fn pool_fee(&self) -> &u64 {
        &self.pool_fee
    }

    fn pool_fee_mut(&mut self) -> &mut u64 {
        &mut self.pool_fee
    }


    fn governance_fee(&self) -> &u64 {
        &self.governance_fee
    }

    fn governance_fee_mut(&mut self) -> &mut u64 {
        &mut self.governance_fee
    }


    fn escrowed_assets(&self) -> &Vec<Uint128> {
        &self.escrowed_assets
    }

    fn escrowed_assets_mut(&mut self) -> &mut Vec<Uint128> {
        &mut self.escrowed_assets
    }


    fn escrowed_pool_tokens(&self) -> &Uint128 {
        &self.escrowed_pool_tokens
    }

    fn escrowed_pool_tokens_mut(&mut self) -> &mut Uint128 {
        &mut self.escrowed_pool_tokens
    }


    fn max_limit_capacity(&self) -> &U256 {
        &self.max_limit_capacity
    }

    fn max_limit_capacity_mut(&mut self) -> &mut U256 {
        &mut self.max_limit_capacity
    }


    fn used_limit_capacity(&self) -> &U256 {
        &self.used_limit_capacity
    }

    fn used_limit_capacity_mut(&mut self) -> &mut U256 {
        &mut self.used_limit_capacity
    }


    fn used_limit_capacity_timestamp(&self) -> &u64 {
        &self.used_limit_capacity_timestamp
    }

    fn used_limit_capacity_timestamp_mut(&mut self) -> &mut u64 {
        &mut self.used_limit_capacity_timestamp
    }

}



impl CatalystV1PoolAdministration for SwapPoolVolatileState {

    fn initialize_swap_curves(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        assets: Vec<String>,
        assets_balances: Vec<Uint128>,  //TODO EVM MISMATCH
        weights: Vec<u64>,
        amp: u64,
        depositor: String
    ) -> Result<Response, swap_pool_common::ContractError> {

        let mut state = STATE.load(deps.storage)?;

        // Check the caller is the Factory
        //TODO verify info sender is Factory

        // Make sure this function may only be invoked once (check whether assets have already been saved)
        if state.assets.len() > 0 {
            return Err(swap_pool_common::ContractError::Unauthorized {});
        }

        // Check that the amplification is correct (set to 1)
        if amp != 10u64.pow(18) {     //TODO maths WAD
            return Err(swap_pool_common::ContractError::InvalidAmplification {})
        }

        // Check the provided assets and weights count
        if
            assets.len() == 0 || assets.len() > MAX_ASSETS ||
            weights.len() != assets.len()
        {
            return Err(swap_pool_common::ContractError::GenericError {}); //TODO error
        }

        // Validate the depositor address
        deps.api.addr_validate(&depositor)?;

        // Validate and save assets
        state.assets = assets
            .iter()
            .map(|asset_addr| deps.api.addr_validate(&asset_addr))
            .collect::<StdResult<Vec<Addr>>>()
            .map_err(|_| swap_pool_common::ContractError::InvalidAssets {})?;

        // Validate asset balances
        if assets_balances.iter().any(|balance| balance.is_zero()) {
            return Err(swap_pool_common::ContractError::GenericError {}); //TODO error
        }

        // Validate and save weights
        if weights.iter().any(|weight| *weight == 0) {
            return Err(swap_pool_common::ContractError::GenericError {}); //TODO error
        }
        state.weights = weights.clone();

        // Compute the security limit
        state.max_limit_capacity = LN2 * weights.iter().fold(
            U256::ZERO, |acc, next| acc + U256::from(*next)     // Overflow safe, as U256 >> u64    //TODO maths
        );

        // Save state
        STATE.save(deps.storage, &state)?;

        // Mint pool tokens for the depositor
        // Make up a 'MessageInfo' with the sender set to this contract itself => this is to allow the use of the 'execute_mint'
        // function as provided by cw20-base, which will match the 'sender' of 'MessageInfo' with the allowed minter that
        // was set when initializing the cw20 token (this contract itself).
        let execute_mint_info = MessageInfo {
            sender: env.contract.address.clone(),
            funds: vec![],
        };
        let minted_amount = INITIAL_MINT_AMOUNT;
        execute_mint(
            deps,
            env.clone(),
            execute_mint_info,
            depositor.clone(),
            minted_amount
        )?;

        // TODO EVM MISMATCH // TODO overhaul: are tokens transferred from the factory? Or will they already be hold by the contract at this point?
        // Build messages to order the transfer of tokens from setup_master to the swap pool
        let sender_addr_str = info.sender.to_string();
        let self_addr_str = env.contract.address.to_string();
        let transfer_msgs: Vec<CosmosMsg> = state.assets.iter().zip(&assets_balances).map(|(asset, balance)| {
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: sender_addr_str.clone(),
                        recipient: self_addr_str.clone(),
                        amount: *balance
                    })?,
                    funds: vec![]
                }
            ))
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;

        //TODO include attributes of the execute_mint response in this response?
        Ok(
            Response::new()
                .add_messages(transfer_msgs)
                .add_attribute("to_account", depositor)
                .add_attribute("mint", minted_amount)
                .add_attribute("assets", format!("{:?}", assets_balances))
        )
    }

}



impl CatalystV1PoolPermissionless for SwapPoolVolatileState {}



impl CatalystV1PoolDerived for SwapPoolVolatileState {

    fn calc_send_asset(
        &self,
        deps: Deps,
        env: Env,
        from_asset: &str,
        amount: Uint128
    ) -> Result<U256, swap_pool_common::ContractError> {

        let from_asset_index: usize = self.get_asset_index(from_asset.as_ref())?;
        let from_asset_balance: Uint128 = deps.querier.query_wasm_smart(
            from_asset,
            &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
        )?;
        let from_asset_weight = self.weights[from_asset_index];

        calc_price_curve_area(
            amount.u128().into(),
            from_asset_balance.u128().into(),
            U256::from(from_asset_weight),
        ).map_err(|_| swap_pool_common::ContractError::GenericError {})
    }

    fn calc_receive_asset(
        &self,
        deps: Deps,
        env: Env,
        to_asset: &str,
        u: U256
    ) -> Result<Uint128, swap_pool_common::ContractError> {

        let to_asset_index: usize = self.get_asset_index(to_asset.as_ref())?;
        let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<Uint128>(
            to_asset,
            &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
        )?.checked_sub(self.escrowed_assets[to_asset_index])?;      // pool balance minus escrowed balance
        let to_asset_weight = self.weights[to_asset_index];
        
        calc_price_curve_limit(
            u,
            to_asset_balance.u128().into(),
            U256::from(to_asset_weight),
        ).map(
            |val| Uint128::from(val.as_u128())
        ).map_err(
            |_| swap_pool_common::ContractError::GenericError {}
        )      //TODO! .as_u128 may overflow silently
    }

    fn calc_local_swap(
        &self,
        deps: Deps,
        env: Env,
        from_asset: &str,
        to_asset: &str,
        amount: Uint128
    ) -> Result<Uint128, swap_pool_common::ContractError> {

        let from_asset_index: usize = self.get_asset_index(from_asset.as_ref())?;
        let from_asset_balance: Uint128 = deps.querier.query_wasm_smart(
            from_asset,
            &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
        )?;
        let from_asset_weight = self.weights[from_asset_index];

        let to_asset_index: usize = self.get_asset_index(to_asset.as_ref())?;
        let to_asset_balance: Uint128 = deps.querier.query_wasm_smart::<Uint128>(
            to_asset,
            &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
        )?.checked_sub(self.escrowed_assets[to_asset_index])?;      // pool balance minus escrowed balance
        let to_asset_weight = self.weights[to_asset_index];

        calc_combined_price_curves(
            amount.u128().into(),
            from_asset_balance.u128().into(),
            to_asset_balance.u128().into(),
            U256::from(from_asset_weight),
            U256::from(to_asset_weight)
        ).map(
            |val| Uint128::from(val.as_u128())
        ).map_err(
            |_| swap_pool_common::ContractError::GenericError {}
        ) 
    }

}


impl CatalystV1PoolAckTimeout for SwapPoolVolatileState {
    fn send_asset_ack(
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        asset: String,
        block_number_mod: u32
    ) -> Result<Response, swap_pool_common::ContractError> {    //TODO error

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

        let used_capacity = state.used_limit_capacity_mut();
        *used_capacity = used_capacity.saturating_sub(u);
    
        state.save_state(deps.storage)?;

        Ok(response)
    }

    fn send_liquidity_ack(
        deps: &mut DepsMut,
        info: MessageInfo,
        to_account: String,
        u: U256,
        amount: Uint128,
        block_number_mod: u32
    ) -> Result<Response, swap_pool_common::ContractError> {    //TODO error

        let mut state = Self::load_state(deps.storage)?;

        let response = state.on_send_liquidity_ack(
            deps,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        )?;

        let used_capacity = state.used_limit_capacity_mut();
        *used_capacity = used_capacity.saturating_sub(u);

        state.save_state(deps.storage)?;

        Ok(response)
    }
}

impl SwapPoolVolatileState {
    
}