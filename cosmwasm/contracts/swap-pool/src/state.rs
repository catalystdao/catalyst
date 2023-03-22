use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg, to_binary, Deps};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use cw20_base::{contract::execute_mint, allowances::execute_burn_from};
use cw_storage_plus::Item;
use ethnum::U256;
use fixed_point_math_lib::fixed_point_math::{LN2, mul_wad_down, self};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use swap_pool_common::{
    state::{
        MAX_ASSETS, INITIAL_MINT_AMOUNT, CatalystV1PoolState, CatalystV1PoolAdministration, CatalystV1PoolAckTimeout, CatalystV1PoolPermissionless, CatalystV1PoolDerived
    },
    ContractError
};

use crate::calculation_helpers::{calc_price_curve_area, calc_price_curve_limit, calc_combined_price_curves, calc_price_curve_limit_share};

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
    pub pool_fee: u64,              // TODO store as U256?
    pub governance_fee: u64,        // TODO store as U256?

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
    ) -> Result<Response, ContractError> {

        let mut state = STATE.load(deps.storage)?;

        // Check the caller is the Factory
        //TODO verify info sender is Factory

        // Make sure this function may only be invoked once (check whether assets have already been saved)
        if state.assets.len() > 0 {
            return Err(ContractError::Unauthorized {});
        }

        // Check that the amplification is correct (set to 1)
        if amp != 10u64.pow(18) {     //TODO maths WAD
            return Err(ContractError::InvalidAmplification {})
        }

        // Check the provided assets and weights count
        if
            assets.len() == 0 || assets.len() > MAX_ASSETS ||
            weights.len() != assets.len()
        {
            return Err(ContractError::GenericError {}); //TODO error
        }

        // Validate the depositor address
        deps.api.addr_validate(&depositor)?;

        // Validate and save assets
        state.assets = assets
            .iter()
            .map(|asset_addr| deps.api.addr_validate(&asset_addr))
            .collect::<StdResult<Vec<Addr>>>()
            .map_err(|_| ContractError::InvalidAssets {})?;

        // Validate asset balances
        if assets_balances.iter().any(|balance| balance.is_zero()) {
            return Err(ContractError::GenericError {}); //TODO error
        }

        // Validate and save weights
        if weights.iter().any(|weight| *weight == 0) {
            return Err(ContractError::GenericError {}); //TODO error
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



impl CatalystV1PoolPermissionless for SwapPoolVolatileState {

    fn deposit_mixed(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        deposit_amounts: Vec<Uint128>,  //TODO EVM MISMATCH
        min_out: Uint128
    ) -> Result<Response, ContractError> {
        
        // Load as not mutable, as no state variable gets modified
        let state = Self::load_state(deps.storage)?;

        // Compute how much 'units' the assets are worth.
        // Iterate over the assets, weights and deposit_amounts)
        let u = state.assets().iter()
            .zip(state.weights())
            .zip(&deposit_amounts)
            .try_fold(U256::ZERO, |acc, ((asset, weight), deposit_amount)| {

                let pool_asset_balance = deps.querier.query_wasm_smart(
                    asset,
                    &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
                )?;

                acc.checked_add(
                    calc_price_curve_area(
                        U256::from(deposit_amount.u128()),
                        pool_asset_balance,
                        U256::from(weight.clone())
                    )?
                ).ok_or(ContractError::ArithmeticError {})
            })?;

        // Subtract the pool fee from U to prevent deposit and withdrawals being employed as a method of swapping.
        // To recude costs, the governance fee is not taken. This is not an issue as swapping via this method is 
        // disincentivized by its higher gas costs.
        let u = fixed_point_math::mul_wad_down(u, fixed_point_math::WAD - U256::from(state.pool_fee))?;

        // Do not include the 'escrowed' pool tokens in the total supply of pool tokens (return less)
        let effective_supply = U256::from(Self::total_supply(deps.as_ref())?.u128());

        // Derive the weight sum (w_sum) from the security limit capacity       //TODO do we want this in this implementation?
        let w_sum = state.max_limit_capacity() / fixed_point_math::LN2;

        // Compute the pool tokens to be minted.
        let out = Uint128::from(fixed_point_math::mul_wad_down(
            effective_supply,                                                       // Note 'effective_supply' is not WAD, hence result will not be either
            calc_price_curve_limit_share(u, w_sum)?
        )?.as_u128());      //TODO OVERFLOW

        // Check that the minimum output is honoured.
        if min_out > out {
            return Err(ContractError::ReturnInsufficient { out, min_out });
        }

        // Mint the pool tokens
        let mint_response = execute_mint(
            deps.branch(),
            env.clone(),
            MessageInfo {
                sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
                funds: vec![],
            },
            info.sender.to_string(),
            out
        )?;

        // Build messages to order the transfer of tokens from the depositor to the swap pool
        let transfer_msgs: Vec<CosmosMsg> = state.assets.iter().zip(&deposit_amounts).map(|(asset, balance)| {
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: *balance
                    })?,
                    funds: vec![]
                }
            ))
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;

        Ok(Response::new()
            .add_messages(transfer_msgs)
            .add_events(mint_response.events)                           // Add mint events //TODO overhaul
            .add_attribute("to_account", info.sender.to_string())
            .add_attribute("mint", out)
            .add_attribute("assets", format!("{:?}", deposit_amounts))  //TODO deposit_amounts event format
        )
    }

    fn withdraw_all(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        pool_tokens: Uint128,
        min_out: Vec<Uint128>,
    ) -> Result<Response, ContractError> {
        
        // Load as not mutable, as no state variable gets modified
        let state = Self::load_state(deps.storage)?;

        // Include the 'escrowed' pool tokens in the total supply of pool tokens of the pool
        let effective_supply = Self::total_supply(deps.as_ref())?.checked_add(state.escrowed_pool_tokens)?;

        // Burn the pool tokens of the withdrawer
        let sender = info.sender.to_string();
        let burn_response = execute_burn_from(deps.branch(), env.clone(), info.clone(), sender.clone(), pool_tokens)?;

        // Compute the withdraw amounts
        let withdraw_amounts: Vec<Uint128> = state.assets()
            .iter()
            .zip(state.escrowed_assets())
            .zip(&min_out)
            .map(|((asset, escrowed_balance), asset_min_out)| {
            
            let pool_asset_balance = deps.querier.query_wasm_smart::<Uint128>(
                asset,
                &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
            )? - escrowed_balance;

            //TODO use U256 for the calculation?
            let withdraw_amount = (pool_asset_balance * pool_tokens) / effective_supply;

            // Check that the minimum output is honoured.
            if *asset_min_out > withdraw_amount {
                return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
            };

            Ok(withdraw_amount)
        }).collect::<Result<Vec<Uint128>, ContractError>>()?;

        // Build messages to order the transfer of tokens from the swap pool to the depositor
        let transfer_msgs: Vec<CosmosMsg> = state.assets.iter().zip(&withdraw_amounts).map(|(asset, amount)| {
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: env.contract.address.to_string(),
                        recipient: sender.clone(),
                        amount: *amount
                    })?,
                    funds: vec![]
                }
            ))
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;


        Ok(Response::new()
            .add_messages(transfer_msgs)
            .add_events(burn_response.events)                           // Add burn events //TODO overhaul
            .add_attribute("to_account", info.sender.to_string())
            .add_attribute("burn", pool_tokens)
            .add_attribute("assets", format!("{:?}", withdraw_amounts))  //TODO withdraw_amounts format
        )
        
    }


    fn withdraw_mixed(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        pool_tokens: Uint128,
        withdraw_ratio: Vec<u64>,
        min_out: Vec<Uint128>,
    ) -> Result<Response, ContractError> {
        
        // Load as not mutable, as no state variable gets modified
        let state = Self::load_state(deps.storage)?;

        // Include the 'escrowed' pool tokens in the total supply of pool tokens of the pool
        let effective_supply = U256::from(
            Self::total_supply(deps.as_ref())?.checked_add(state.escrowed_pool_tokens)?.u128()
        );

        // Burn the pool tokens of the withdrawer
        let sender = info.sender.to_string();
        let burn_response = execute_burn_from(deps.branch(), env.clone(), info.clone(), sender.clone(), pool_tokens)?;

        // Derive the weight sum (w_sum) from the security limit capacity       //TODO do we want this in this implementation?
        let w_sum = state.max_limit_capacity() / fixed_point_math::LN2;

        // Compute the unit worth of the pool tokens.
        let mut u: U256 = fixed_point_math::ln_wad(
            fixed_point_math::div_wad_down(
                effective_supply,
                effective_supply - U256::from(pool_tokens.u128())  // Subtraction is underflow safe, as the above 'execute_burn_from' guarantees that 'pool_tokens' is contained in 'effective_supply'
            )?.as_i256()                                           // Casting my overflow to a negative value. In that case, 'ln_wad' will fail.
        )?.as_u256()                                               // Casting is safe, as ln is computed of values >= 1, hence output is always positive
            .checked_mul(w_sum).ok_or(ContractError::ArithmeticError {})?;

        // Compute the withdraw amounts
        let withdraw_amounts: Vec<Uint128> = state.assets()
            .iter()
            .zip(state.weights())
            .zip(state.escrowed_assets())
            .zip(&withdraw_ratio)
            .zip(&min_out)
            .map(|((((asset, weight), escrowed_balance), asset_withdraw_ratio), asset_min_out)| {

                // Calculate the units allocated for the specific asset
                let units_for_asset = fixed_point_math::mul_wad_down(u, U256::from(*asset_withdraw_ratio))?;
                if units_for_asset == U256::ZERO {

                    // There should not be a non-zero withdraw ratio after a withdraw ratio of 1 (protect against user error)
                    if *asset_withdraw_ratio != 0 {
                        return Err(ContractError::WithdrawRatioNotZero { ratio: *asset_withdraw_ratio }) 
                    };

                    // Check that the minimum output is honoured.
                    if asset_min_out != Uint128::zero() {
                        return Err(ContractError::ReturnInsufficient { out: Uint128::zero(), min_out: *asset_min_out })
                    };

                    return Ok(Uint128::zero());
                }

                // Subtract the units used from the total units amount. This will underflow for malicious withdraw ratios (i.e. ratios > 1).
                u = u.checked_sub(units_for_asset).ok_or(ContractError::ArithmeticError {})?;
            
                // Get the pool asset balance (subtract the escrowed assets to return less)
                let pool_asset_balance = deps.querier.query_wasm_smart::<Uint128>(
                    asset,
                    &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
                )? - escrowed_balance;

                // Calculate the asset amount corresponding to the asset units
                let withdraw_amount = Uint128::from(
                    calc_price_curve_limit(
                        units_for_asset,
                        U256::from(pool_asset_balance.u128()),
                        U256::from(*weight)
                    )?.as_u128()        // TODO unsafe overflow
                );

                // Check that the minimum output is honoured.
                if *asset_min_out > withdraw_amount {
                    return Err(ContractError::ReturnInsufficient { out: withdraw_amount.clone(), min_out: *asset_min_out });
                };

                Ok(withdraw_amount)
            }).collect::<Result<Vec<Uint128>, ContractError>>()?;

        // Make sure all units have been consumed
        if u != U256::ZERO { return Err(ContractError::UnusedUnitsAfterWithdrawal { units: u }) };       //TODO error

        // Build messages to order the transfer of tokens from the swap pool to the depositor
        let transfer_msgs: Vec<CosmosMsg> = state.assets.iter().zip(&withdraw_amounts).map(|(asset, amount)| {
            Ok(CosmosMsg::Wasm(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: asset.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: env.contract.address.to_string(),
                        recipient: sender.clone(),
                        amount: *amount
                    })?,
                    funds: vec![]
                }
            ))
        }).collect::<StdResult<Vec<CosmosMsg>>>()?;


        Ok(Response::new()
            .add_messages(transfer_msgs)
            .add_events(burn_response.events)                           // Add burn events //TODO overhaul
            .add_attribute("to_account", info.sender.to_string())
            .add_attribute("burn", pool_tokens)
            .add_attribute("assets", format!("{:?}", withdraw_amounts))  //TODO withdraw_amounts format
        )
        
    }

    fn local_swap(
        deps: &Deps,
        env: Env,
        info: MessageInfo,
        from_asset: String,
        to_asset: String,
        amount: Uint128,
        min_out: Uint128
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        state.update_weights()?;

        let pool_fee: Uint128 = mul_wad_down(            //TODO alternative to not have to use U256 conversion? (or wrapper?)
            U256::from(amount.u128()),
            U256::from(*state.pool_fee())
        )?.as_u128().into();    // Casting safe, as fee < amount, and amount is Uint128

        // Calculate the return value
        let out: Uint128 = state.calc_local_swap(
            *deps,
            env.clone(),
            &from_asset,
            &to_asset,
            amount - pool_fee
        )?;

        if min_out > out {
            return Err(ContractError::ReturnInsufficient { out, min_out });
        }

        // Build message to transfer input assets to the pool
        let transfer_from_asset_msg = CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: from_asset.clone(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount
                })?,
                funds: vec![]
            }
        );

        // Build message to transfer output assets to the swapper
        let transfer_to_asset_msg = CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: to_asset.clone(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.contract.address.to_string(),
                    recipient: info.sender.to_string(),
                    amount: out
                })?,
                funds: vec![]
            }
        );

        // Build collect governance fee message
        let collect_governance_fee_message = state.collect_governance_fee_message(
            env,
            from_asset.clone(),
            pool_fee
        )?;

        Ok(Response::new()
            .add_message(transfer_from_asset_msg)
            .add_message(transfer_to_asset_msg)
            .add_message(collect_governance_fee_message)
            .add_attribute("to_account", info.sender.to_string())
            .add_attribute("from_asset", from_asset)
            .add_attribute("to_asset", to_asset)
            .add_attribute("from_amount", amount)
            .add_attribute("to_amount", out)
        )
    }


    fn send_asset(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        channel_id: String,
        to_pool: String,
        to_account: String,
        from_asset: String,
        to_asset_index: u8,
        amount: Uint128,
        min_out: Uint128,
        fallback_account: String,   //TODO EVM mismatch
        calldata: Vec<u8>
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        // Only allow connected pools
        if !SwapPoolVolatileState::is_connected(&deps.as_ref(), &channel_id, &to_pool) {
            return Err(ContractError::PoolNotConnected { channel_id, pool: to_pool })
        }

        state.update_weights()?;

        let pool_fee: Uint128 = mul_wad_down(            //TODO alternative to not have to use U256 conversion? (or wrapper?)
            U256::from(amount.u128()),
            U256::from(*state.pool_fee())
        )?.as_u128().into();    // Casting safe, as fee < amount, and amount is Uint128

        // Calculate the group-specific units bought
        let u = state.calc_send_asset(
            deps.as_ref(),
            env.clone(),
            &from_asset,
            amount - pool_fee
        )?;

        let send_asset_hash = SwapPoolVolatileState::compute_send_asset_hash(
            to_account.as_str(),
            u,
            amount - pool_fee,
            &from_asset,
            env.block.height as u32
        );

        //TODO invoke interface

        state.create_asset_escrow(
            deps,
            &send_asset_hash,
            amount - pool_fee,
            &from_asset,
            fallback_account
        )?;

        // Build message to transfer input assets to the pool
        let transfer_from_asset_msg = CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: from_asset.clone(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount
                })?,
                funds: vec![]
            }
        );

        // Build collect governance fee message
        let collect_governance_fee_message = state.collect_governance_fee_message(
            env,
            from_asset.clone(),
            pool_fee
        )?;

        state.save_state(deps.storage)?;    //TODO Is this only needed if the weights are updated?

        Ok(Response::new()
            .add_message(transfer_from_asset_msg)
            .add_message(collect_governance_fee_message)
            .add_attribute("to_pool", to_pool)
            .add_attribute("to_account", info.sender.to_string())
            .add_attribute("from_asset", from_asset)
            .add_attribute("to_asset_index", to_asset_index.to_string())
            .add_attribute("from_amount", amount)
            .add_attribute("units", u.to_string())
            .add_attribute("min_out", min_out)
            .add_attribute("swap_hash", send_asset_hash)
        )
    }

    fn receive_asset(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        channel_id: String,
        from_pool: String,
        to_asset_index: u8,
        to_account: String,
        u: U256,
        min_out: Uint128,
        swap_hash: String,
        calldata: Vec<u8>   //TODO calldata
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        // Only allow connected pools
        if !SwapPoolVolatileState::is_connected(&deps.as_ref(), &channel_id, &from_pool) {
            return Err(ContractError::PoolNotConnected { channel_id, pool: from_pool })
        }

        if Some(info.sender) != *state.chain_interface() {
            return Err(ContractError::Unauthorized {});
        }

        state.update_weights()?;

        let to_asset = state.assets
            .get(to_asset_index as usize)
            .ok_or(ContractError::GenericError {})?
            .clone(); //TODO error

        state.update_unit_capacity(env.block.time, u)?;

        let out = state.calc_receive_asset(deps.as_ref(), env.clone(), to_asset.as_str(), u)?;
        

        if min_out > out {
            return Err(ContractError::ReturnInsufficient { out, min_out });
        }

        // Build message to transfer output assets to to_account
        let transfer_to_asset_msg = CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: to_asset.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.contract.address.to_string(),
                    recipient: to_account.to_string(),
                    amount: out
                })?,
                funds: vec![]
            }
        );

        Ok(Response::new()
            .add_message(transfer_to_asset_msg)
            .add_attribute("from_pool", from_pool)
            .add_attribute("to_account", to_account)
            .add_attribute("to_asset", to_asset)
            .add_attribute("units", u.to_string())  //TODO format of .to_string()?
            .add_attribute("to_amount", out)
            .add_attribute("swap_hash", swap_hash)
        )
    }

    fn send_liquidity(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        channel_id: String,
        to_pool: String,
        to_account: String,
        amount: Uint128,            //TODO EVM mismatch
        min_out: Uint128,
        fallback_account: String,   //TODO EVM mismatch
        calldata: Vec<u8>
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        // Only allow connected pools
        if !SwapPoolVolatileState::is_connected(&deps.as_ref(), &channel_id, &to_pool) {
            return Err(ContractError::PoolNotConnected { channel_id, pool: to_pool })
        }

        // Update weights
        state.update_weights()?;

        // Include the 'escrowed' pool tokens in the total supply of pool tokens of the pool
        let effective_supply = U256::from(Self::total_supply(deps.as_ref())?.u128()) 
            + U256::from(state.escrowed_pool_tokens.u128());        // Addition is overflow safe because of casting into U256

        // Burn the pool tokens of the sender
        let sender = info.sender.to_string();
        execute_burn_from(deps.branch(), env.clone(), info, sender, amount)?;

        // Derive the weight sum (w_sum) from the security limit capacity       //TODO do we want this in this implementation?
        let w_sum = state.max_limit_capacity() / fixed_point_math::LN2;

        // Compute the unit value of the provided poolTokens
        // This step simplifies withdrawing and swapping into a single step
        let u = fixed_point_math::ln_wad(
            fixed_point_math::div_wad_down(
                effective_supply,
                effective_supply - U256::from(amount.u128())   // subtraction is safe, as 'amount' is always contained in 'effective_supply'
            )?.as_i256()                                         // if casting overflows into a negative value, posterior 'ln' calc will fail
        )?.as_u256()                                         // casting safe as 'ln' is computed of a value >= 1 (hence result always positive)
            .checked_mul(w_sum)
            .ok_or(ContractError::ArithmeticError {})?;

        // Compute the hash of the 'send_liquidity' transaction
        let send_liquidity_hash = SwapPoolVolatileState::compute_send_liquidity_hash(
            to_account.as_str(),
            u,
            amount,
            env.block.height as u32
        );


        //TODO invoke interface


        // Escrow the pool tokens
        state.create_liquidity_escrow(
            deps,
            &send_liquidity_hash,
            amount,
            fallback_account
        )?;

        state.save_state(deps.storage)?;    //TODO Is this only needed if the weights are updated?

        Ok(Response::new()
            .add_attribute("to_pool", to_pool)
            .add_attribute("to_account", to_account)
            .add_attribute("from_amount", amount)
            .add_attribute("units", u.to_string())
            .add_attribute("swap_hash", send_liquidity_hash)
        )
    }

    fn receive_liquidity(
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        channel_id: String,
        from_pool: String,
        to_account: String,
        u: U256,
        min_out: Uint128,
        swap_hash: String,
        calldata: Vec<u8>   //TODO calldata
    ) -> Result<Response, ContractError> {

        let mut state = Self::load_state(deps.storage)?;

        // Only allow connected pools
        if !SwapPoolVolatileState::is_connected(&deps.as_ref(), &channel_id, &from_pool) {
            return Err(ContractError::PoolNotConnected { channel_id, pool: from_pool })
        }

        if Some(info.sender) != *state.chain_interface() {
            return Err(ContractError::Unauthorized {});
        }

        state.update_weights()?;

        state.update_unit_capacity(env.block.time, u)?;

        // Derive the weight sum (w_sum) from the security limit capacity       //TODO do we want this in this implementation?
        let w_sum = state.max_limit_capacity() / fixed_point_math::LN2;

        // Do not include the 'escrowed' pool tokens in the total supply of pool tokens of the pool (return less)
        let effective_supply = U256::from(Self::total_supply(deps.as_ref())?.u128());
    
        // Use 'calc_price_curve_limit_share' to get the % of pool tokens that should be minted (in WAD terms)
        // Multiply by 'effective_supply' to get the absolute amount (not in WAD terms) using 'mul_wad_down' so
        // that the result is also NOT in WAD terms.
        let out = fixed_point_math::mul_wad_down(
            calc_price_curve_limit_share(u, w_sum)?,
            effective_supply
        ).map(|val| Uint128::from(val.as_u128()))?;     //TODO OVERFLOW when casting U256 to Uint128. Theoretically calc_price_curve_limit_share < 1, hence casting is safe
    
        if min_out > out {
            return Err(ContractError::ReturnInsufficient { out, min_out });
        }

        // Validate the to_account
        deps.api.addr_validate(&to_account)?;

        // Mint the pool tokens
        let mint_response = execute_mint(
            deps.branch(),
            env.clone(),
            MessageInfo {
                sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
                funds: vec![],
            },
            to_account.clone(),
            out
        )?;

        Ok(Response::new()
            .add_attribute("from_pool", from_pool)
            .add_attribute("to_account", to_account)
            .add_attribute("units", u.to_string())  //TODO format of .to_string()?
            .add_attribute("to_amount", out)
            .add_attribute("swap_hash", swap_hash)
            .add_events(mint_response.events)       //TODO overhaul
        )
    }

}



impl CatalystV1PoolDerived for SwapPoolVolatileState {

    fn calc_send_asset(
        &self,
        deps: Deps,
        env: Env,
        from_asset: &str,
        amount: Uint128
    ) -> Result<U256, ContractError> {

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
        ).map_err(|_| ContractError::GenericError {})
    }

    fn calc_receive_asset(
        &self,
        deps: Deps,
        env: Env,
        to_asset: &str,
        u: U256
    ) -> Result<Uint128, ContractError> {

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
            |_| ContractError::GenericError {}
        )      //TODO! .as_u128 may overflow silently
    }

    fn calc_local_swap(
        &self,
        deps: Deps,
        env: Env,
        from_asset: &str,
        to_asset: &str,
        amount: Uint128
    ) -> Result<Uint128, ContractError> {

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
            |_| ContractError::GenericError {}
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

        let used_capacity = state.used_limit_capacity_mut();
        *used_capacity = used_capacity.saturating_sub(u);

        state.save_state(deps.storage)?;

        Ok(response)
    }
}


impl SwapPoolVolatileState {

    fn update_weights(&mut self) -> Result<(), ContractError> {
        todo!()
    }
    
}