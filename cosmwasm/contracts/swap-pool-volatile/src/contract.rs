#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
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
    send_asset_timeout, send_liquidity_ack, send_liquidity_timeout, query_chain_interface, query_setup_master, query_ready, query_only_local, query_assets, query_weights, query_pool_fee, query_governance_fee_share, query_fee_administrator, query_total_escrowed_liquidity, query_total_escrowed_asset, query_asset_escrow, query_liquidity_escrow, query_pool_connection_state, query_factory, query_factory_owner
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
    info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, ContractError> {

    //TODO move to 'setup'
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
        msg.setup_master,
        info.sender                 //TODO EVM mismatch/review: setting the 'info.sender' as the 'factory'
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
            weights,
            amp,
            depositor
        } => initialize_swap_curves(
            &mut deps,
            env,
            info,
            assets,
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
            calldata_target,
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
            calldata_target,
            calldata
        ),

        VolatileExecuteMsg::SendLiquidity {
            channel_id,
            to_pool,
            to_account,
            amount,
            min_pool_tokens,
            min_reference_asset,
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
            min_pool_tokens,
            min_reference_asset,
            fallback_account,
            calldata
        ),

        VolatileExecuteMsg::ReceiveLiquidity {
            channel_id,
            from_pool,
            to_account,
            u,
            min_pool_tokens,
            min_reference_asset,
            calldata_target,
            calldata
        } => receive_liquidity(
            &mut deps,
            env,
            info,
            channel_id,
            from_pool,
            to_account,
            u,
            min_pool_tokens,
            min_reference_asset,
            calldata_target,
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
        QueryMsg::Factory {} => to_binary(&query_factory(deps)?),
        QueryMsg::FactoryOwner {} => to_binary(&query_factory_owner(deps)?),

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
        QueryMsg::AssetEscrow { hash } => to_binary(&query_asset_escrow(deps, hash)?),
        QueryMsg::LiquidityEscrow { hash } => to_binary(&query_liquidity_escrow(deps, hash)?),

        // Volatile-Specific Queries
        QueryMsg::TargetWeights {} => to_binary(&query_target_weights(deps)?),
        QueryMsg::WeightsUpdateFinishTimestamp {} => to_binary(&query_weights_update_finish_timestamp(deps)?),

        // CW20 query msgs - Use cw20-base for the implementation
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => to_binary(&query_allowance(deps, owner, spender)?)
    }
}
