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
use swap_pool_common::ContractError;
use swap_pool_common::state::{
    setup, finish_setup, set_fee_administrator, set_pool_fee, set_governance_fee_share, set_connection, send_asset_ack,
    send_asset_timeout, send_liquidity_ack, send_liquidity_timeout, query_chain_interface, query_setup_master, query_ready, query_only_local, query_assets, query_weights, query_pool_fee, query_governance_fee_share, query_fee_administrator, query_total_escrowed_liquidity, query_total_escrowed_asset, query_asset_escrow, query_liquidity_escrow, query_pool_connection_state
};

use crate::msg::{VolatileExecuteMsg, InstantiateMsg, QueryMsg, VolatileExecuteExtension};
use crate::state::{
    initialize_swap_curves, set_weights, deposit_mixed, withdraw_all, withdraw_mixed, local_swap, send_asset, receive_asset,
    send_liquidity, receive_liquidity, query_calc_send_asset, query_calc_receive_asset, query_calc_local_swap, query_get_limit_capacity, query_target_weights, query_weights_update_finish_timestamp
};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-swap-pool-volatile";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(
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
    msg: VolatileExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {

        VolatileExecuteMsg::InitializeSwapCurves {
            assets,
            assets_balances,
            weights,
            amp,
            depositor
        } => initialize_swap_curves(
            &mut deps,
            env,
            info,
            assets,
            assets_balances,
            weights,
            amp,
            depositor
        ),

        VolatileExecuteMsg::FinishSetup {} => finish_setup(
            &mut deps,
            info
        ),

        VolatileExecuteMsg::SetFeeAdministrator { administrator } => set_fee_administrator(
            &mut deps,
            info,
            administrator
        ),

        VolatileExecuteMsg::SetPoolFee { fee } => set_pool_fee(
            &mut deps,
            info,
            fee
        ),

        VolatileExecuteMsg::SetGovernanceFeeShare { fee } => set_governance_fee_share(
            &mut deps,
            info,
            fee
        ),

        VolatileExecuteMsg::SetConnection {
            channel_id,
            to_pool,
            state
        } => set_connection(
            &mut deps,
            info,
            channel_id,
            to_pool,
            state
        ),

        VolatileExecuteMsg::SendAssetAck {
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        } => send_asset_ack(
            &mut deps,
            info,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        ),

        VolatileExecuteMsg::SendAssetTimeout {
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        } => send_asset_timeout(
            &mut deps,
            env,
            info,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        ),

        VolatileExecuteMsg::SendLiquidityAck {
            to_account,
            u,
            amount,
            block_number_mod
        } => send_liquidity_ack(
            &mut deps,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        ),

        VolatileExecuteMsg::SendLiquidityTimeout {
            to_account,
            u,
            amount,
            block_number_mod
        } => send_liquidity_timeout(
            &mut deps,
            env,
            info,
            to_account,
            u,
            amount,
            block_number_mod
        ),

        VolatileExecuteMsg::DepositMixed {
            deposit_amounts,
            min_out
        } => deposit_mixed(
            &mut deps,
            env,
            info,
            deposit_amounts,
            min_out
        ),

        VolatileExecuteMsg::WithdrawAll {
            pool_tokens,
            min_out
        } => withdraw_all(
            &mut deps,
            env,
            info,
            pool_tokens,
            min_out
        ),

        VolatileExecuteMsg::WithdrawMixed {
            pool_tokens,
            withdraw_ratio,
            min_out
        } => withdraw_mixed(
            &mut deps,
            env,
            info,
            pool_tokens,
            withdraw_ratio,
            min_out
        ),

        VolatileExecuteMsg::LocalSwap {
            from_asset,
            to_asset,
            amount,
            min_out
        } => local_swap(
            &mut deps,
            env,
            info,
            from_asset,
            to_asset,
            amount,
            min_out
        ),

        VolatileExecuteMsg::SendAsset {
            channel_id,
            to_pool,
            to_account,
            from_asset,
            to_asset_index,
            amount,
            min_out,
            fallback_account,
            calldata
        } => send_asset(
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

        VolatileExecuteMsg::ReceiveAsset {
            channel_id,
            from_pool,
            to_asset_index,
            to_account,
            u,
            min_out,
            swap_hash,
            calldata
        } => receive_asset(
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

        VolatileExecuteMsg::SendLiquidity {
            channel_id,
            to_pool,
            to_account,
            amount,
            min_out,
            fallback_account,
            calldata
        } => send_liquidity(
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

        VolatileExecuteMsg::ReceiveLiquidity {
            channel_id,
            from_pool,
            to_account,
            u,
            min_out,
            swap_hash,
            calldata
        } => receive_liquidity(
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

        VolatileExecuteMsg::Custom(VolatileExecuteExtension::SetWeights {
            weights,
            target_timestamp
        }) => set_weights(
            &mut deps,
            &env,
            weights,
            target_timestamp
        ),


        // CW20 execute msgs - Use cw20-base for the implementation
        VolatileExecuteMsg::Transfer {
            recipient,
            amount
        } => Ok(
            execute_transfer(deps, env, info, recipient, amount)?
        ),

        VolatileExecuteMsg::Burn {
            amount: _
         } => Err(
            ContractError::Unauthorized {}     // Pool token burn handled by withdraw function
        ),

        VolatileExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(
            execute_send(deps, env, info, contract, amount, msg)?
        ),

        VolatileExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_increase_allowance(deps, env, info, spender, amount, expires)?
        ),

        VolatileExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_decrease_allowance(deps, env, info, spender, amount, expires)?
        ),

        VolatileExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(
            execute_transfer_from(deps, env, info, owner, recipient, amount)?
        ),

        VolatileExecuteMsg::BurnFrom {
            owner: _,
            amount: _
        } => Err(
            ContractError::Unauthorized {}      // Pool token burn handled by withdraw function
        ),

        VolatileExecuteMsg::SendFrom {
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

        // Common Queries
        QueryMsg::ChainInterface {} => to_binary(&query_chain_interface(deps)?),
        QueryMsg::SetupMaster {} => to_binary(&query_setup_master(deps)?),

        QueryMsg::PoolConnectionState {
            channel_id,
            pool 
        } => to_binary(&query_pool_connection_state(deps, channel_id.as_ref(), pool)?),

        QueryMsg::Ready{} => to_binary(&query_ready(deps)?),
        QueryMsg::OnlyLocal{} => to_binary(&query_only_local(deps)?),
        QueryMsg::Assets {} => to_binary(&query_assets(deps)?),
        QueryMsg::Weights {} => to_binary(&query_weights(deps)?),

        QueryMsg::PoolFee {} => to_binary(&query_pool_fee(deps)?),
        QueryMsg::GovernanceFeeShare {} => to_binary(&query_governance_fee_share(deps)?),
        QueryMsg::FeeAdministrator {} => to_binary(&query_fee_administrator(deps)?),

        QueryMsg::CalcSendAsset{
            from_asset,
            amount
        } => to_binary(&query_calc_send_asset(deps,env, &from_asset,amount)?),
        QueryMsg::CalcReceiveAsset{
            to_asset,
            u
        } => to_binary(&query_calc_receive_asset(deps,env, &to_asset,u)?),
        QueryMsg::CalcLocalSwap{
            from_asset,
            to_asset,
            amount
        } => to_binary(&query_calc_local_swap(deps,env, &from_asset, &to_asset,amount)?),

        QueryMsg::GetLimitCapacity{} => to_binary(&query_get_limit_capacity(deps,env)?),

        QueryMsg::TotalEscrowedAsset {
            asset
        } => to_binary(&query_total_escrowed_asset(deps, asset.as_ref())?),
        QueryMsg::TotalEscrowedLiquidity {} => to_binary(&query_total_escrowed_liquidity(deps)?),
        QueryMsg::AssetEscrow { hash } => to_binary(&query_asset_escrow(deps, hash.as_str())?),
        QueryMsg::LiquidityEscrow { hash } => to_binary(&query_liquidity_escrow(deps, hash.as_str())?),

        // Volatile-Specific Queries
        QueryMsg::TargetWeights {} => to_binary(&query_target_weights(deps)?),
        QueryMsg::WeightsUpdateFinishTimestamp {} => to_binary(&query_weights_update_finish_timestamp(deps)?),

        // CW20 query msgs - Use cw20-base for the implementation
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => to_binary(&query_allowance(deps, owner, spender)?)
    }
}




#[cfg(test)]
mod tests {
    use cw_multi_test::{App, Contract, ContractWrapper, Executor};

    use cosmwasm_std::{Addr, Empty, Uint128, Attribute};
    use cw20::{Cw20Coin, Cw20ExecuteMsg, MinterResponse, Cw20QueryMsg, BalanceResponse};
    use swap_pool_common::{msg::InstantiateMsg, state::INITIAL_MINT_AMOUNT};

    use crate::msg::VolatileExecuteMsg;

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
        let initialize_balances_msg = VolatileExecuteMsg::InitializeSwapCurves {
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
