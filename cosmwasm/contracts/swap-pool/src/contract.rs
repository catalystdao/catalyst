#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, StdError};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary, Uint128};
use cw2::set_contract_version;
use cw20_base::allowances::{
    execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_send, execute_transfer, query_balance, query_token_info,
};
use ethnum::U256;
use swap_pool_common::{
    state::{CatalystV1PoolAdministration, CatalystV1PoolAckTimeout, CatalystV1PoolPermissionless, CatalystV1PoolState, CatalystV1PoolDerived},
    ContractError
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::SwapPoolVolatileState;


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-swap-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    SwapPoolVolatileState::setup(
        &mut deps,
        &env,
        msg.name,
        msg.symbol,
        msg.chain_interface,
        msg.pool_fee,
        msg.governance_fee,
        msg.fee_administrator,
        msg.setup_master
    )

}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {

        ExecuteMsg::InitializeSwapCurves {
            assets,
            assets_balances,
            weights,
            amp,
            depositor
        } => SwapPoolVolatileState::initialize_swap_curves(
            deps,
            env,
            info,
            assets,
            assets_balances,
            weights,
            amp,
            depositor
        ),

        ExecuteMsg::FinishSetup {} => SwapPoolVolatileState::finish_setup(
            &mut deps,
            info
        ),

        ExecuteMsg::SetFeeAdministrator { administrator } => SwapPoolVolatileState::set_fee_administrator(
            &mut deps,
            info,
            administrator
        ),

        ExecuteMsg::SetPoolFee { fee } => SwapPoolVolatileState::set_pool_fee(
            &mut deps,
            info,
            fee
        ),

        ExecuteMsg::SetGovernanceFee { fee } => SwapPoolVolatileState::set_governance_fee(
            &mut deps,
            info,
            fee
        ),

        ExecuteMsg::SetConnection {
            channel_id,
            to_pool,
            state
        } => SwapPoolVolatileState::set_connection(
            &mut deps,
            info,
            channel_id,
            to_pool,
            state
        ),

        ExecuteMsg::SetWeights {
            weights,
            target_timestamp
        } => SwapPoolVolatileState::set_weights(
            &mut deps,
            &env,
            weights,
            target_timestamp
        ),

        ExecuteMsg::SendAssetAck {
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        } => SwapPoolVolatileState::send_asset_ack(
            &mut deps,
            info,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        ),

        ExecuteMsg::SendAssetTimeout {
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        } => SwapPoolVolatileState::send_asset_timeout(
            &mut deps,
            env,
            info,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        ),

        ExecuteMsg::SendLiquidityAck {
            to_account,
            u,
            amount,
            block_number_mod
        } => SwapPoolVolatileState::send_liquidity_ack(
            &mut deps,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        ),

        ExecuteMsg::SendLiquidityTimeout {
            to_account,
            u,
            amount,
            block_number_mod
        } => SwapPoolVolatileState::send_liquidity_timeout(
            &mut deps,
            env,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        ),

        ExecuteMsg::DepositMixed {
            deposit_amounts,
            min_out
        } => SwapPoolVolatileState::deposit_mixed(
            &mut deps,
            env,
            info,
            deposit_amounts,
            min_out
        ),

        ExecuteMsg::WithdrawAll {
            pool_tokens,
            min_out
        } => SwapPoolVolatileState::withdraw_all(
            &mut deps,
            env,
            info,
            pool_tokens,
            min_out
        ),

        ExecuteMsg::WithdrawMixed {
            pool_tokens,
            withdraw_ratio,
            min_out
        } => SwapPoolVolatileState::withdraw_mixed(
            &mut deps,
            env,
            info,
            pool_tokens,
            withdraw_ratio,
            min_out
        ),

        ExecuteMsg::LocalSwap {
            from_asset,
            to_asset,
            amount,
            min_out
        } => SwapPoolVolatileState::local_swap(
            &deps.as_ref(),
            env,
            info,
            from_asset,
            to_asset,
            amount,
            min_out
        ),

        ExecuteMsg::SendAsset {
            channel_id,
            to_pool,
            to_account,
            from_asset,
            to_asset_index,
            amount,
            min_out,
            fallback_account,
            calldata
        } => SwapPoolVolatileState::send_asset(
            &mut deps,
            env,
            info,
            channel_id,
            to_pool,
            to_account,
            from_asset,
            to_asset_index,
            amount,
            min_out,
            fallback_account,
            calldata
        ),

        ExecuteMsg::ReceiveAsset {
            channel_id,
            from_pool,
            to_asset_index,
            to_account,
            u,
            min_out,
            swap_hash,
            calldata
        } => SwapPoolVolatileState::receive_asset(
            &mut deps,
            env,
            info,
            channel_id,
            from_pool,
            to_asset_index,
            to_account,
            u,
            min_out,
            swap_hash,
            calldata
        ),

        ExecuteMsg::SendLiquidity {
            channel_id,
            to_pool,
            to_account,
            amount,
            min_out,
            fallback_account,
            calldata
        } => SwapPoolVolatileState::send_liquidity(
            &mut deps,
            env,
            info,
            channel_id,
            to_pool,
            to_account,
            amount,
            min_out,
            fallback_account,
            calldata
        ),

        ExecuteMsg::ReceiveLiquidity {
            channel_id,
            from_pool,
            to_account,
            u,
            min_out,
            swap_hash,
            calldata
        } => SwapPoolVolatileState::receive_liquidity(
            &mut deps,
            env,
            info,
            channel_id,
            from_pool,
            to_account,
            u,
            min_out,
            swap_hash,
            calldata
        ),


        // CW20 execute msgs - Use cw20-base for the implementation
        ExecuteMsg::Transfer {
            recipient,
            amount
        } => Ok(
            execute_transfer(deps, env, info, recipient, amount)?
        ),

        ExecuteMsg::Burn {
            amount: _
         } => Err(
            ContractError::Unauthorized {}     // Pool token burn handled by withdraw function
        ),

        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(
            execute_send(deps, env, info, contract, amount, msg)?
        ),

        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_increase_allowance(deps, env, info, spender, amount, expires)?
        ),

        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_decrease_allowance(deps, env, info, spender, amount, expires)?
        ),

        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(
            execute_transfer_from(deps, env, info, owner, recipient, amount)?
        ),

        ExecuteMsg::BurnFrom {
            owner: _,
            amount: _
        } => Err(
            ContractError::Unauthorized {}      // Pool token burn handled by withdraw function
        ),

        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(
            execute_send_from(deps, env, info, owner, contract, amount, msg)?
        ),
    }
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ready {} => to_binary(&query_ready(deps)?),
        QueryMsg::OnlyLocal {} => to_binary(&query_only_local(deps)?),
        QueryMsg::GetUnitCapacity {} => to_binary(&query_get_unit_capacity(deps, env)?),

        QueryMsg::CalcSendAsset { from_asset, amount } => to_binary(
            &query_calc_send_asset(deps, env, &from_asset, amount)?
        ),
        QueryMsg::CalcReceiveAsset { to_asset, u } => to_binary(
            &query_calc_receive_asset(deps, env, &to_asset, u)?
        ),
        QueryMsg::CalcLocalSwap { from_asset, to_asset, amount } => to_binary(
            &query_calc_local_swap(deps, env, &from_asset, &to_asset, amount)?
        ),

        // CW20 query msgs - Use cw20-base for the implementation
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => to_binary(&query_allowance(deps, owner, spender)?)
    }
}




// pub fn execute_deposit(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     pool_tokens_amount: Uint128
// ) -> Result<Response, ContractError> {

//     let mut state = STATE.load(deps.storage)?;

//     let pool_token_supply = get_pool_token_supply(deps.as_ref())?;

//     // Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
//     // upwards by depositing changes the limit.
//     let current_timestamp: u64 = env.block.time.seconds();
//     state.update_liquidity_units_inflow(
//         Uint128::zero(),    // No pool tokens are going into the pool via a bridge
//         pool_token_supply,
//         current_timestamp
//     )?;
    
//     // Given the desired 'pool_tokens_amount', compute the corresponding deposit amounts for each of the assets of the pool
//     let balance_msg = Cw20QueryMsg::Balance { address: env.contract.address.to_string() };

//     let deposited_amounts: Vec<Uint128> = state.assets.iter().enumerate().map(|(asset_index, asset)| {

//         // Get the pool's asset balance
//         let swap_pool_asset_balance = deps.querier.query_wasm_smart(
//             asset,
//             &balance_msg
//         )?;

//         // Determine the share of the common 'pool_tokens_amount' that corresponds to this asset
//         let asset_balance0 = state.assets_balance0s[asset_index];
//         let pool_tokens_for_asset = pool_tokens_amount
//             .checked_mul(asset_balance0).map_err(|_| ContractError::ArithmeticError {})?
//             .checked_div(pool_token_supply).map_err(|_| ContractError::ArithmeticError {})?;

//         // Determine the asset deposit amout
//         let asset_deposit_amount = calculation_helpers::calc_asset_amount_for_pool_tokens(
//             pool_tokens_for_asset,
//             swap_pool_asset_balance,     // Escrowed tokens are NOT subtracted from the total balance => deposits should return less
//             asset_balance0
//         )?;
        
//         // Update the asset balance0
//         state.assets_balance0s[asset_index] = 
//             asset_balance0.checked_add(pool_tokens_for_asset).map_err(|_| ContractError::ArithmeticError {})?;
        
//         Ok(asset_deposit_amount)

//     }).collect::<Result<Vec<Uint128>, ContractError>>()?;

//     // Save state
//     STATE.save(deps.storage, &state)?;


//     // Mint pool tokens for the depositor
//     let depositor_addr_str = info.sender.to_string();
//     let self_addr_str = env.contract.address.to_string();

//     // Make up a 'MessageInfo' with the sender set to this contract itself => this is to allow the use of the 'execute_mint'
//     // function as provided by cw20-base, which will match the 'sender' of 'MessageInfo' with the allowed minter that
//     // was set when initializing the cw20 token (this contract itself).
//     let execute_mint_info = MessageInfo {
//         sender: env.contract.address.clone(),
//         funds: vec![],
//     };
//     execute_mint(deps, env.clone(), execute_mint_info, self_addr_str.clone(), pool_tokens_amount)?;


//     // TODO move transfer functionality somewhere else? Implement on 'SwapPoolState'?
//     // Build messages to order the transfer of tokens from the depositor to the swap pool
//     let transfer_msgs: Vec<CosmosMsg> = state.assets.iter().zip(&deposited_amounts).map(|(asset, balance)| {
//         Ok(CosmosMsg::Wasm(
//             cosmwasm_std::WasmMsg::Execute {
//                 contract_addr: asset.to_string(),
//                 msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
//                     owner: depositor_addr_str.clone(),
//                     recipient: self_addr_str.clone(),
//                     amount: *balance
//                 })?,
//                 funds: vec![]
//             }
//         ))
//     }).collect::<StdResult<Vec<CosmosMsg>>>()?;

//     Ok(
//         Response::new()
//             .add_messages(transfer_msgs)
//             .add_attribute("deposited_amounts", format!("{:?}", deposited_amounts))
//             .add_attribute("minted_pool_tokens", pool_tokens_amount)
//     )
// }


// pub fn execute_withdraw(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     pool_tokens_amount: Uint128
// ) -> Result<Response, ContractError> {

//     let mut state = STATE.load(deps.storage)?;

//     let pool_token_supply = get_pool_token_supply(deps.as_ref())?;

//     // Update the liqudity security limit. Since the limit is based on the current totalSupply, changing the totalSupply
//     // downwards by withdrawing changes the limit.
//     let current_timestamp: u64 = env.block.time.seconds();
//     state.update_liquidity_units_inflow(
//         Uint128::zero(),    // No pool tokens are going into the pool via a bridge
//         pool_token_supply,
//         current_timestamp
//     )?;
    
//     // Given the desired 'pool_tokens_amount', compute the corresponding withdraw amounts for each of the assets of the pool
//     let balance_msg = Cw20QueryMsg::Balance { address: env.contract.address.to_string() };

//     let withdrawal_amounts: Vec<Uint128> = state.assets.iter().enumerate().map(|(asset_index, asset)| {

//         // Get the pool's asset balance
//         let swap_pool_asset_balance: Uint128 = deps.querier.query_wasm_smart(
//             asset,
//             &balance_msg
//         )?;

//         // Determine the share of the common 'pool_tokens_amount' that corresponds to this asset
//         let asset_balance0 = state.assets_balance0s[asset_index];
//         let pool_tokens_for_asset = pool_tokens_amount
//             .checked_mul(asset_balance0).map_err(|_| ContractError::ArithmeticError {})?
//             .checked_div(pool_token_supply).map_err(|_| ContractError::ArithmeticError {})?;

//         // Determine the asset withdrawal amount
//         let asset_withdrawal_amount = calculation_helpers::calc_asset_amount_for_pool_tokens(
//             pool_tokens_for_asset,
//             swap_pool_asset_balance
//                 .checked_sub(state.escrowed_assets[asset_index]).map_err(|_| ContractError::ArithmeticError {})?,         // Escrowed tokens ARE subtracted from the total balance => withdrawals should return less
//             asset_balance0
//         )?;
        
//         // Update the asset balance0
//         state.assets_balance0s[asset_index] = 
//             asset_balance0.checked_sub(pool_tokens_for_asset).map_err(|_| ContractError::ArithmeticError {})?;
        
//         Ok(asset_withdrawal_amount)

//     }).collect::<Result<Vec<Uint128>, ContractError>>()?;

//     // Save state
//     STATE.save(deps.storage, &state)?;



//     // Burn pool tokens from the withdrawer
//     let withdrawer_addr_str = info.sender.to_string();
//     let self_addr_str = env.contract.address.to_string();

//     execute_burn_from(deps, env.clone(), info, withdrawer_addr_str.clone(), pool_tokens_amount)?;

    
//     // TODO move transfer functionality somewhere else? Implement on 'SwapPoolState'?
//     // Build messages to order the transfer of tokens from the depositor to the swap pool
//     let transfer_msgs: Vec<CosmosMsg> = state.assets.iter().zip(&withdrawal_amounts).map(|(asset, balance)| {
//         Ok(CosmosMsg::Wasm(
//             cosmwasm_std::WasmMsg::Execute {
//                 contract_addr: asset.to_string(),
//                 msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
//                     owner: self_addr_str.clone(),
//                     recipient: withdrawer_addr_str.clone(),
//                     amount: *balance
//                 })?,
//                 funds: vec![]
//             }
//         ))
//     }).collect::<StdResult<Vec<CosmosMsg>>>()?;

    
//     Ok(
//         Response::new()
//             .add_messages(transfer_msgs)
//             .add_attribute("withdrawn_amounts", format!("{:?}", withdrawal_amounts))
//             .add_attribute("burnt_pool_tokens", pool_tokens_amount)
//     )

// }





pub fn query_ready(deps: Deps) -> StdResult<bool> {
    SwapPoolVolatileState::ready(deps)
}


pub fn query_only_local(deps: Deps) -> StdResult<bool> {
    SwapPoolVolatileState::only_local(deps)
}

pub fn query_get_unit_capacity(deps: Deps, env: Env) -> StdResult<U256> { //TODO maths
    SwapPoolVolatileState::get_unit_capacity(deps, env)
        .map(|capacity| capacity)
        .map_err(|_| StdError::GenericErr { msg: "".to_owned() })   //TODO error
}


pub fn query_calc_send_asset(
    deps: Deps,
    env: Env,
    from_asset: &str,
    amount: Uint128
) -> StdResult<U256> {

    SwapPoolVolatileState::load_state(deps.storage)?
        .calc_send_asset(deps, env, from_asset, amount)
        .map_err(|err| err.into())

}


pub fn query_calc_receive_asset(
    deps: Deps,
    env: Env,
    to_asset: &str,
    u: U256
) -> StdResult<Uint128> {

    SwapPoolVolatileState::load_state(deps.storage)?
        .calc_receive_asset(deps, env, to_asset, u)
        .map_err(|err| err.into())

}


pub fn query_calc_local_swap(
    deps: Deps,
    env: Env,
    from_asset: &str,
    to_asset: &str,
    amount: Uint128
) -> StdResult<Uint128> {

    SwapPoolVolatileState::load_state(deps.storage)?
        .calc_local_swap(deps, env, from_asset, to_asset, amount)
        .map_err(|err| err.into())

}



#[cfg(test)]
mod tests {
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    use cosmwasm_std::{Addr, Empty, Uint128, Attribute};
    use cw20::{Cw20Coin, Cw20ExecuteMsg, MinterResponse, Cw20QueryMsg, BalanceResponse};
    use swap_pool_common::{msg::{InstantiateMsg, ExecuteMsg}, state::INITIAL_MINT_AMOUNT};

    pub const INSTANTIATOR_ADDR: &str = "inst_addr";
    pub const OTHER_ADDR: &str = "other_addr";

    pub fn contract_swap_pool() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query
        );
        Box::new(contract)
    }

    //TODO add instantiate tests

    #[test]
    fn test_setup_and_instantiate() {
        //TODO should this be considered an integration test? => Move somewhere else

        let mut router = App::default();

        let setup_master = Addr::unchecked(INSTANTIATOR_ADDR);


        // Create test token
        let cw20_id = router.store_code(contract_cw20());

        let msg = cw20_base::msg::InstantiateMsg {
            name: "Test Token A".to_string(),
            symbol: "TTA".to_string(),
            decimals: 2,
            initial_balances: vec![Cw20Coin {
                address: setup_master.to_string(),
                amount: Uint128::new(5000)
            }],
            mint: Some(MinterResponse {
                minter: setup_master.to_string(),
                cap: None
            }),
            marketing: None
        };

        let test_token_1_addr = router
            .instantiate_contract(cw20_id, setup_master.clone(), &msg, &[], "TTA", None)
            .unwrap();


        // Create swap pool - upload and instantiate
        let sp_id = router.store_code(contract_swap_pool());

        let sp_addr = router
            .instantiate_contract(
                sp_id,
                setup_master.clone(),
                &InstantiateMsg {
                    name: "Pool1".to_owned(),
                    symbol: "P1".to_owned(),
                    chain_interface: None,
                    pool_fee: 0u64,
                    governance_fee: 0u64,
                    fee_administrator: setup_master.to_string(),
                    setup_master: setup_master.to_string(),
                },
                &[],
                "sp",
                None
            ).unwrap();



        // Set allowance for the swap pool
        let deposit_amount = Uint128::from(1000_u64);
        let allowance_msg = Cw20ExecuteMsg::IncreaseAllowance {
            spender: sp_addr.to_string(),
            amount: deposit_amount,
            expires: None
        };

        router.execute_contract(
            setup_master.clone(),
            test_token_1_addr.clone(),
            &allowance_msg,
            &[]
        ).unwrap();


        // Initialize sp balances
        let initialize_balances_msg = ExecuteMsg::InitializeSwapCurves {
            assets: vec![test_token_1_addr.to_string()],
            assets_balances: vec![Uint128::from(1000_u64)],
            weights: vec![1u64],
            amp: 1000000000000000000u64,
            depositor: setup_master.to_string()
        };

        let response = router.execute_contract(
            setup_master.clone(),
            sp_addr.clone(),
            &initialize_balances_msg,
            &[]
        ).unwrap();

        // Verify attributes
        let initialize_event = response.events[1].clone();
        assert_eq!(
            initialize_event.attributes[1],
            Attribute { key: "to_account".to_string(), value: setup_master.to_string()}
        );
        assert_eq!(
            initialize_event.attributes[2],
            Attribute { key: "mint".to_string(), value: INITIAL_MINT_AMOUNT.to_string()}
        );
        assert_eq!(
            initialize_event.attributes[3],
            Attribute { key: "assets".to_string(), value: format!("{:?}", vec![Uint128::from(1000_u64)])}
        );


        // Verify token balances
        // Swap pool balance of test token 1
        let balance_msg = Cw20QueryMsg::Balance { address: sp_addr.to_string() };
        let balance_response: BalanceResponse = router
            .wrap()
            .query_wasm_smart(test_token_1_addr.clone(), &balance_msg)
            .unwrap();
        assert_eq!(balance_response.balance, Uint128::from(1000_u64));

        // User balance of test token 1
        let balance_msg = Cw20QueryMsg::Balance { address: setup_master.to_string() };
        let balance_response: BalanceResponse = router
            .wrap()
            .query_wasm_smart(test_token_1_addr.clone(), &balance_msg)
            .unwrap();

        assert_eq!(balance_response.balance, Uint128::from(4000_u64));


        // Verify pool token balance
        let balance_msg = Cw20QueryMsg::Balance { address: setup_master.to_string() };
        let balance_response: BalanceResponse = router
            .wrap()
            .query_wasm_smart(sp_addr.clone(), &balance_msg)
            .unwrap();

        assert_eq!(balance_response.balance, INITIAL_MINT_AMOUNT);

    }

    // TODO make sure 'InitializeSwapCurves' can only be called once

}
