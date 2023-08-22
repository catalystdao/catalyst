#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use cw2::set_contract_version;
use cw20_base::allowances::{
    execute_decrease_allowance, execute_increase_allowance, execute_send_from, execute_transfer_from, query_allowance,
};
use cw20_base::contract::{
    execute_send, execute_transfer, query_balance, query_token_info,
};
use catalyst_vault_common::ContractError;
use catalyst_vault_common::state::{
    setup, finish_setup, set_fee_administrator, set_vault_fee, set_governance_fee_share, set_connection, on_send_asset_failure, on_send_liquidity_failure, query_chain_interface, query_setup_master, query_ready, query_only_local, query_assets, query_weight, query_vault_fee, query_governance_fee_share, query_fee_administrator, query_total_escrowed_liquidity, query_total_escrowed_asset, query_asset_escrow, query_liquidity_escrow, query_vault_connection_state, query_factory, query_factory_owner
};

use crate::msg::{VolatileExecuteMsg, InstantiateMsg, QueryMsg, VolatileExecuteExtension};
use crate::state::{
    initialize_swap_curves, set_weights, deposit_mixed, withdraw_all, withdraw_mixed, local_swap, send_asset, receive_asset, send_liquidity, receive_liquidity, query_calc_send_asset, query_calc_receive_asset, query_calc_local_swap, query_get_limit_capacity, query_target_weight, query_weights_update_finish_timestamp, on_send_asset_success_volatile, on_send_liquidity_success_volatile
};

// Version information
const CONTRACT_NAME: &str = "catalyst-vault-volatile";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



// Instantiation **********************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(
        &mut deps,
        &env,
        info,
        msg.name,
        msg.symbol,
        msg.chain_interface,
        msg.vault_fee,
        msg.governance_fee_share,
        msg.fee_administrator,
        msg.setup_master
    )

}



// Execution **************************************************************************************

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

        VolatileExecuteMsg::FinishSetup {
        } => finish_setup(
            &mut deps,
            info
        ),

        VolatileExecuteMsg::SetFeeAdministrator {
            administrator
        } => set_fee_administrator(
            &mut deps,
            info,
            administrator
        ),

        VolatileExecuteMsg::SetVaultFee {
            fee
        } => set_vault_fee(
            &mut deps,
            info,
            fee
        ),

        VolatileExecuteMsg::SetGovernanceFeeShare {
            fee
        } => set_governance_fee_share(
            &mut deps,
            info,
            fee
        ),

        VolatileExecuteMsg::SetConnection {
            channel_id,
            to_vault,
            state
        } => set_connection(
            &mut deps,
            info,
            channel_id,
            to_vault,
            state
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
            vault_tokens,
            min_out
        } => withdraw_all(
            &mut deps,
            env,
            info,
            vault_tokens,
            min_out
        ),

        VolatileExecuteMsg::WithdrawMixed {
            vault_tokens,
            withdraw_ratio,
            min_out
        } => withdraw_mixed(
            &mut deps,
            env,
            info,
            vault_tokens,
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
            to_vault,
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
            to_vault,
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
            from_vault,
            to_asset_index,
            to_account,
            u,
            min_out,
            from_amount,
            from_asset,
            from_block_number_mod,
            calldata_target,
            calldata
        } => receive_asset(
            &mut deps,
            env,
            info,
            channel_id,
            from_vault,
            to_asset_index,
            to_account,
            u,
            min_out,
            from_amount,
            from_asset,
            from_block_number_mod,
            calldata_target,
            calldata
        ),

        VolatileExecuteMsg::SendLiquidity {
            channel_id,
            to_vault,
            to_account,
            amount,
            min_vault_tokens,
            min_reference_asset,
            fallback_account,
            calldata
        } => send_liquidity(
            &mut deps,
            env,
            info,
            channel_id,
            to_vault,
            to_account,
            amount,
            min_vault_tokens,
            min_reference_asset,
            fallback_account,
            calldata
        ),

        VolatileExecuteMsg::ReceiveLiquidity {
            channel_id,
            from_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            calldata_target,
            from_amount,
            from_block_number_mod,
            calldata
        } => receive_liquidity(
            &mut deps,
            env,
            info,
            channel_id,
            from_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            from_block_number_mod,
            calldata_target,
            calldata
        ),

        VolatileExecuteMsg::OnSendAssetSuccess {
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset,
            block_number_mod
        } => on_send_asset_success_volatile(        // ! Use the volatile specific 'on_send_asset_success'
            &mut deps,
            &env,
            &info,
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset,
            block_number_mod
        ),

        VolatileExecuteMsg::OnSendAssetFailure {
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset,
            block_number_mod
        } => on_send_asset_failure(
            &mut deps,
            &env,
            &info,
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset,
            block_number_mod
        ),

        VolatileExecuteMsg::OnSendLiquiditySuccess {
            channel_id,
            to_account,
            u,
            escrow_amount,
            block_number_mod
        } => on_send_liquidity_success_volatile(    // ! Use the volatile specific 'on_send_liquidity_success'
            &mut deps,
            &env,
            &info,
            channel_id,
            to_account,
            u,
            escrow_amount,
            block_number_mod
        ),

        VolatileExecuteMsg::OnSendLiquidityFailure {
            channel_id,
            to_account,
            u,
            escrow_amount,
            block_number_mod
        } => on_send_liquidity_failure(
            &mut deps,
            &env,
            &info,
            channel_id,
            to_account,
            u,
            escrow_amount,
            block_number_mod
        ),

        VolatileExecuteMsg::Custom(extension) => {

            match extension {
                VolatileExecuteExtension::SetWeights {
                    target_timestamp,
                    new_weights
                } => set_weights(
                    &mut deps,
                    &env,
                    info,
                    target_timestamp,
                    new_weights
                ),
            }

        },


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
            ContractError::Unauthorized {}     // Vault token burn handled by withdraw function
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
            ContractError::Unauthorized {}      // Vault token burn handled by withdraw function
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



// Query ******************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {

        // Common Queries
        QueryMsg::ChainInterface {} => to_binary(&query_chain_interface(deps)?),
        QueryMsg::SetupMaster {} => to_binary(&query_setup_master(deps)?),
        QueryMsg::Factory {} => to_binary(&query_factory(deps)?),
        QueryMsg::FactoryOwner {} => to_binary(&query_factory_owner(deps)?),

        QueryMsg::VaultConnectionState {
            channel_id,
            vault 
        } => to_binary(&query_vault_connection_state(deps, channel_id.as_ref(), vault)?),

        QueryMsg::Ready{} => to_binary(&query_ready(deps)?),
        QueryMsg::OnlyLocal{} => to_binary(&query_only_local(deps)?),
        QueryMsg::Assets {} => to_binary(&query_assets(deps)?),
        QueryMsg::Weight {
            asset
        } => to_binary(&query_weight(deps, asset)?),

        QueryMsg::VaultFee {} => to_binary(&query_vault_fee(deps)?),
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
        QueryMsg::TargetWeight {
            asset
        } => to_binary(&query_target_weight(deps, asset)?),
        QueryMsg::WeightsUpdateFinishTimestamp {} => to_binary(&query_weights_update_finish_timestamp(deps)?),

        // CW20 query msgs - Use cw20-base for the implementation
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => to_binary(&query_allowance(deps, owner, spender)?)
    }
}
