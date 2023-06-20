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
use catalyst_vault_common::ContractError;
use catalyst_vault_common::state::{
    setup, finish_setup, set_fee_administrator, set_vault_fee, set_governance_fee_share, set_connection,
    query_chain_interface, query_setup_master, query_ready, query_only_local, query_assets, query_weight, query_vault_fee, query_governance_fee_share, query_fee_administrator, query_total_escrowed_liquidity, query_total_escrowed_asset, query_asset_escrow, query_liquidity_escrow, query_vault_connection_state, query_factory, query_factory_owner, on_send_liquidity_success
};

use crate::msg::{AmplifiedExecuteMsg, InstantiateMsg, QueryMsg, AmplifiedExecuteExtension};
use crate::state::{
    initialize_swap_curves, deposit_mixed, withdraw_all, withdraw_mixed, local_swap, send_asset, receive_asset,
    send_liquidity, receive_liquidity, query_calc_send_asset, query_calc_receive_asset, query_calc_local_swap, query_get_limit_capacity, on_send_asset_success_amplified, on_send_asset_failure_amplified, on_send_liquidity_failure_amplified, set_amplification, query_target_amplification, query_amplification_update_finish_timestamp
};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-vault-amplified";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(
        &mut deps,
        &env,
        msg.name,
        msg.symbol,
        msg.chain_interface,
        msg.vault_fee,
        msg.governance_fee_share,
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
    msg: AmplifiedExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {

        AmplifiedExecuteMsg::InitializeSwapCurves {
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

        AmplifiedExecuteMsg::FinishSetup {} => finish_setup(
            &mut deps,
            info
        ),

        AmplifiedExecuteMsg::SetFeeAdministrator { administrator } => set_fee_administrator(
            &mut deps,
            info,
            administrator
        ),

        AmplifiedExecuteMsg::SetVaultFee { fee } => set_vault_fee(
            &mut deps,
            info,
            fee
        ),

        AmplifiedExecuteMsg::SetGovernanceFeeShare { fee } => set_governance_fee_share(
            &mut deps,
            info,
            fee
        ),

        AmplifiedExecuteMsg::SetConnection {
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

        AmplifiedExecuteMsg::OnSendAssetSuccess {
            channel_id,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        } => on_send_asset_success_amplified(        // ! Use the amplified specific 'on_send_asset_success'
            &mut deps,
            info,
            channel_id,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        ),

        AmplifiedExecuteMsg::OnSendAssetFailure {
            channel_id,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        } => on_send_asset_failure_amplified(       // ! Use the amplified specific 'on_send_asset_failure'
            &mut deps,
            info,
            channel_id,
            to_account,
            u,
            amount,
            asset,
            block_number_mod
        ),

        AmplifiedExecuteMsg::OnSendLiquiditySuccess {
            channel_id,
            to_account,
            u,
            amount,
            block_number_mod
        } => on_send_liquidity_success(
            &mut deps,
            info,
            channel_id,
            to_account,
            u,
            amount,
            block_number_mod
        ),

        AmplifiedExecuteMsg::OnSendLiquidityFailure {
            channel_id,
            to_account,
            u,
            amount,
            block_number_mod
        } => on_send_liquidity_failure_amplified(   // ! Use the amplified specific 'on_send_liquidity_failure'
            &mut deps,
            env,
            info,
            channel_id,
            to_account,
            u,
            amount,
            block_number_mod
        ),

        AmplifiedExecuteMsg::DepositMixed {
            deposit_amounts,
            min_out
        } => deposit_mixed(
            &mut deps,
            env,
            info,
            deposit_amounts,
            min_out
        ),

        AmplifiedExecuteMsg::WithdrawAll {
            vault_tokens,
            min_out
        } => withdraw_all(
            &mut deps,
            env,
            info,
            vault_tokens,
            min_out
        ),

        AmplifiedExecuteMsg::WithdrawMixed {
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

        AmplifiedExecuteMsg::LocalSwap {
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

        AmplifiedExecuteMsg::SendAsset {
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

        AmplifiedExecuteMsg::ReceiveAsset {
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

        AmplifiedExecuteMsg::SendLiquidity {
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

        AmplifiedExecuteMsg::ReceiveLiquidity {
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

        AmplifiedExecuteMsg::Custom(extension) => {

            match extension {
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp,
                    target_amplification
                } => set_amplification(
                    &mut deps,
                    &env,
                    info,
                    target_timestamp,
                    target_amplification
                )
            }

        },


        // CW20 execute msgs - Use cw20-base for the implementation
        AmplifiedExecuteMsg::Transfer {
            recipient,
            amount
        } => Ok(
            execute_transfer(deps, env, info, recipient, amount)?
        ),

        AmplifiedExecuteMsg::Burn {
            amount: _
         } => Err(
            ContractError::Unauthorized {}     // Vault token burn handled by withdraw function
        ),

        AmplifiedExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(
            execute_send(deps, env, info, contract, amount, msg)?
        ),

        AmplifiedExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_increase_allowance(deps, env, info, spender, amount, expires)?
        ),

        AmplifiedExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_decrease_allowance(deps, env, info, spender, amount, expires)?
        ),

        AmplifiedExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(
            execute_transfer_from(deps, env, info, owner, recipient, amount)?
        ),

        AmplifiedExecuteMsg::BurnFrom {
            owner: _,
            amount: _
        } => Err(
            ContractError::Unauthorized {}      // Vault token burn handled by withdraw function
        ),

        AmplifiedExecuteMsg::SendFrom {
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

        // Amplified-Specific Queries
        QueryMsg::TargetAmplification {} => to_binary(&query_target_amplification(deps)?),
        QueryMsg::AmplificationUpdateFinishTimestamp {} => to_binary(&query_amplification_update_finish_timestamp(deps)?),

        // CW20 query msgs - Use cw20-base for the implementation
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => to_binary(&query_allowance(deps, owner, spender)?)
    }
}
