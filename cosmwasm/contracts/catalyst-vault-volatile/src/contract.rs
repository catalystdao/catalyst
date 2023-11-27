#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, StdResult, to_json_binary};
use cw2::set_contract_version;
use catalyst_vault_common::ContractError;
use catalyst_vault_common::state::{
    setup, finish_setup, set_fee_administrator, set_vault_fee, set_governance_fee_share, set_connection, on_send_asset_failure, on_send_liquidity_failure, query_chain_interface, query_setup_master, query_ready, query_only_local, query_assets, query_weight, query_vault_fee, query_governance_fee_share, query_fee_administrator, query_total_escrowed_liquidity, query_total_escrowed_asset, query_asset_escrow, query_liquidity_escrow, query_vault_connection_state, query_factory, query_factory_owner, query_total_supply, query_balance, query_asset, query_asset_by_index
};
use catalyst_vault_common::bindings::{VaultResponse, VaultAssets, VaultAssetsTrait};

#[cfg(feature="asset_native")]
use catalyst_vault_common::state::query_vault_token_denom;

#[cfg(feature="asset_cw20")]
use cw20_base::allowances::{
    execute_decrease_allowance, execute_increase_allowance, execute_send_from, execute_transfer_from, query_allowance,
};
#[cfg(feature="asset_cw20")]
use cw20_base::contract::{
    execute_send, execute_transfer, query_token_info,
};
#[cfg(feature="asset_cw20")]
use catalyst_vault_common::bindings::IntoVaultResponse;


use crate::msg::{VolatileExecuteMsg, InstantiateMsg, QueryMsg, VolatileExecuteExtension};
use crate::state::{
    initialize_swap_curves, set_weights, deposit_mixed, withdraw_all, withdraw_mixed, local_swap, send_asset, receive_asset, send_liquidity, receive_liquidity, query_calc_send_asset, query_calc_receive_asset, query_calc_local_swap, query_get_limit_capacity, query_target_weight, query_weights_update_finish_timestamp, on_send_asset_success_volatile, on_send_liquidity_success_volatile, underwrite_asset, release_underwrite_asset, delete_underwrite_asset
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
) -> Result<VaultResponse, ContractError> {

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
) -> Result<VaultResponse, ContractError> {

    let mut receive_no_assets = true;

    let result = match msg {

        VolatileExecuteMsg::InitializeSwapCurves {
            assets,
            weights,
            amp,
            depositor
        } => initialize_swap_curves(
            &mut deps,
            env,
            info.clone(),
            assets,
            weights,
            amp,
            depositor
        ),

        VolatileExecuteMsg::FinishSetup {
        } => finish_setup(
            &mut deps,
            info.clone()
        ),

        VolatileExecuteMsg::SetFeeAdministrator {
            administrator
        } => set_fee_administrator(
            &mut deps,
            info.clone(),
            administrator
        ),

        VolatileExecuteMsg::SetVaultFee {
            fee
        } => set_vault_fee(
            &mut deps,
            info.clone(),
            fee
        ),

        VolatileExecuteMsg::SetGovernanceFeeShare {
            fee
        } => set_governance_fee_share(
            &mut deps,
            info.clone(),
            fee
        ),

        VolatileExecuteMsg::SetConnection {
            channel_id,
            to_vault,
            state
        } => set_connection(
            &mut deps,
            info.clone(),
            channel_id,
            to_vault,
            state
        ),

        VolatileExecuteMsg::DepositMixed {
            deposit_amounts,
            min_out
        } => {
            receive_no_assets = false;
            deposit_mixed(
                &mut deps,
                env,
                info.clone(),
                deposit_amounts,
                min_out
            )
        },

        VolatileExecuteMsg::WithdrawAll {
            vault_tokens,
            min_out
        } => withdraw_all(
            &mut deps,
            env,
            info.clone(),
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
            info.clone(),
            vault_tokens,
            withdraw_ratio,
            min_out
        ),

        VolatileExecuteMsg::LocalSwap {
            from_asset_ref,
            to_asset_ref,
            amount,
            min_out
        } => {
            receive_no_assets = false;
            local_swap(
                &mut deps,
                env,
                info.clone(),
                from_asset_ref,
                to_asset_ref,
                amount,
                min_out
            )
        },

        VolatileExecuteMsg::SendAsset {
            channel_id,
            to_vault,
            to_account,
            from_asset_ref,
            to_asset_index,
            amount,
            min_out,
            fallback_account,
            underwrite_incentive_x16,
            calldata,
            incentive
        } => {
            receive_no_assets = false;
            send_asset(
                &mut deps,
                env,
                info.clone(),
                channel_id,
                to_vault,
                to_account,
                from_asset_ref,
                to_asset_index,
                amount,
                min_out,
                None,
                fallback_account,
                underwrite_incentive_x16,
                calldata,
                incentive
            )
        },

        VolatileExecuteMsg::SendAssetFixedUnits {
            channel_id,
            to_vault,
            to_account,
            from_asset_ref,
            to_asset_index,
            amount,
            min_out,
            u,
            fallback_account,
            underwrite_incentive_x16,
            calldata,
            incentive
        } => {
            receive_no_assets = false;
            send_asset(
                &mut deps,
                env,
                info.clone(),
                channel_id,
                to_vault,
                to_account,
                from_asset_ref,
                to_asset_index,
                amount,
                min_out,
                Some(u),
                fallback_account,
                underwrite_incentive_x16,
                calldata,
                incentive
            )
        },

        VolatileExecuteMsg::ReceiveAsset {
            channel_id,
            from_vault,
            to_asset_index,
            to_account,
            u,
            min_out,
            from_amount,
            from_asset,
            from_block_number_mod
        } => receive_asset(
            &mut deps,
            env,
            info.clone(),
            channel_id,
            from_vault,
            to_asset_index,
            to_account,
            u,
            min_out,
            from_amount,
            from_asset,
            from_block_number_mod
        ),

        VolatileExecuteMsg::UnderwriteAsset {
            identifier,
            asset_ref,
            u,
            min_out
        } => underwrite_asset(
            &mut deps,
            env,
            info.clone(),
            identifier,
            asset_ref,
            u,
            min_out
        ),

        VolatileExecuteMsg::ReleaseUnderwriteAsset {
            channel_id,
            from_vault,
            identifier,
            asset_ref,
            escrow_amount,
            recipient
        } => release_underwrite_asset(
            &mut deps,
            env,
            info.clone(),
            channel_id,
            from_vault,
            identifier,
            asset_ref,
            escrow_amount,
            recipient
        ),

        VolatileExecuteMsg::DeleteUnderwriteAsset {
            identifier,
            asset_ref,
            u,
            escrow_amount
        } => delete_underwrite_asset(
            &mut deps,
            env,
            info.clone(),
            identifier,
            asset_ref,
            u,
            escrow_amount
        ),

        VolatileExecuteMsg::SendLiquidity {
            channel_id,
            to_vault,
            to_account,
            amount,
            min_vault_tokens,
            min_reference_asset,
            fallback_account,
            calldata,
            incentive
        } => {
            receive_no_assets = false;  // Required for incentive payment
            send_liquidity(
                &mut deps,
                env,
                info.clone(),
                channel_id,
                to_vault,
                to_account,
                amount,
                min_vault_tokens,
                min_reference_asset,
                fallback_account,
                calldata,
                incentive
            )
        },

        VolatileExecuteMsg::ReceiveLiquidity {
            channel_id,
            from_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            from_block_number_mod
        } => receive_liquidity(
            &mut deps,
            env,
            info.clone(),
            channel_id,
            from_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            from_block_number_mod
        ),

        VolatileExecuteMsg::OnSendAssetSuccess {
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset_ref,
            block_number_mod
        } => on_send_asset_success_volatile(        // ! Use the volatile specific 'on_send_asset_success'
            &mut deps,
            &env,
            &info,
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset_ref,
            block_number_mod
        ),

        VolatileExecuteMsg::OnSendAssetFailure {
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset_ref,
            block_number_mod
        } => on_send_asset_failure(
            &mut deps,
            &env,
            &info,
            channel_id,
            to_account,
            u,
            escrow_amount,
            asset_ref,
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
                    info.clone(),
                    target_timestamp,
                    new_weights
                ),
            }

        },


        // CW20 execute msgs - Use cw20-base for the implementation
        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::Transfer {
            recipient,
            amount
        } => Ok(
            execute_transfer(deps, env, info.clone(), recipient, amount)?
                .into_vault_response()
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::Burn {
            amount: _
         } => Err(
            ContractError::Unauthorized {}     // Vault token burn handled by withdraw function
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(
            execute_send(deps, env, info.clone(), contract, amount, msg)?
                .into_vault_response()
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_increase_allowance(deps, env, info.clone(), spender, amount, expires)?
                .into_vault_response()
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(
            execute_decrease_allowance(deps, env, info.clone(), spender, amount, expires)?
                .into_vault_response()
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(
            execute_transfer_from(deps, env, info.clone(), owner, recipient, amount)?
                .into_vault_response()
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::BurnFrom {
            owner: _,
            amount: _
        } => Err(
            ContractError::Unauthorized {}      // Vault token burn handled by withdraw function
        ),

        #[cfg(feature="asset_cw20")]
        VolatileExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(
            execute_send_from(deps, env, info.clone(), owner, contract, amount, msg)?
            .into_vault_response()
        ),
    };

    if receive_no_assets {
        VaultAssets::receive_no_assets(&info)?;
    }

    result
}



// Query ******************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {

        // Common Queries
        QueryMsg::ChainInterface {} => to_json_binary(&query_chain_interface(deps)?),
        QueryMsg::SetupMaster {} => to_json_binary(&query_setup_master(deps)?),
        QueryMsg::Factory {} => to_json_binary(&query_factory(deps)?),
        QueryMsg::FactoryOwner {} => to_json_binary(&query_factory_owner(deps)?),

        QueryMsg::VaultConnectionState {
            channel_id,
            vault 
        } => to_json_binary(&query_vault_connection_state(deps, &channel_id, vault)?),

        QueryMsg::Ready{} => to_json_binary(&query_ready(deps)?),
        QueryMsg::OnlyLocal{} => to_json_binary(&query_only_local(deps)?),
        QueryMsg::Assets {} => to_json_binary(&query_assets(deps)?),
        QueryMsg::Asset {
            asset_ref
        } => to_json_binary(&query_asset(deps, asset_ref)?),
        QueryMsg::AssetByIndex {
            asset_index
        } => to_json_binary(&query_asset_by_index(deps, asset_index)?),
        QueryMsg::Weight {
            asset_ref
        } => to_json_binary(&query_weight(deps, asset_ref)?),

        QueryMsg::TotalSupply {} => to_json_binary(&query_total_supply(deps)?),
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),

        QueryMsg::VaultFee {} => to_json_binary(&query_vault_fee(deps)?),
        QueryMsg::GovernanceFeeShare {} => to_json_binary(&query_governance_fee_share(deps)?),
        QueryMsg::FeeAdministrator {} => to_json_binary(&query_fee_administrator(deps)?),

        QueryMsg::CalcSendAsset{
            from_asset_ref,
            amount
        } => to_json_binary(&query_calc_send_asset(deps, env, from_asset_ref, amount)?),
        QueryMsg::CalcReceiveAsset{
            to_asset_ref,
            u
        } => to_json_binary(&query_calc_receive_asset(deps, env, to_asset_ref, u)?),
        QueryMsg::CalcLocalSwap{
            from_asset_ref,
            to_asset_ref,
            amount
        } => to_json_binary(&query_calc_local_swap(deps, env, from_asset_ref, to_asset_ref, amount)?),

        QueryMsg::GetLimitCapacity{} => to_json_binary(&query_get_limit_capacity(deps, env)?),

        QueryMsg::TotalEscrowedAsset {
            asset_ref
        } => to_json_binary(&query_total_escrowed_asset(deps, asset_ref)?),
        QueryMsg::TotalEscrowedLiquidity {} => to_json_binary(&query_total_escrowed_liquidity(deps)?),
        QueryMsg::AssetEscrow { hash } => to_json_binary(&query_asset_escrow(deps, hash)?),
        QueryMsg::LiquidityEscrow { hash } => to_json_binary(&query_liquidity_escrow(deps, hash)?),

        // Volatile-Specific Queries
        QueryMsg::TargetWeight {
            asset_ref
        } => to_json_binary(&query_target_weight(deps, asset_ref)?),
        QueryMsg::WeightsUpdateFinishTimestamp {} => to_json_binary(&query_weights_update_finish_timestamp(deps)?),

        // Native asset query msgs
        #[cfg(feature="asset_native")]
        QueryMsg::VaultTokenDenom {} => to_json_binary(&query_vault_token_denom(deps)?),

        // CW20 query msgs - Use cw20-base for the implementation
        #[cfg(feature="asset_cw20")]
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        #[cfg(feature="asset_cw20")]
        QueryMsg::Allowance { owner, spender } => to_json_binary(&query_allowance(deps, owner, spender)?)
    }
}
