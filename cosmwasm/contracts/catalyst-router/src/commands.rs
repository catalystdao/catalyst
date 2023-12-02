use catalyst_types::{U256, Bytes32};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CosmosMsg, Deps, Env, Binary, Uint128, Uint64};
use generalised_incentives_common::state::IncentiveDescription;

use crate::{error::ContractError, executors::{catalyst, payments, cancel_swap, types::{CoinAmount, Denom, Account, ValueAmount}}};



/// Commands Encoding
// ************************************************************************************************

#[cw_serde]
pub struct CommandOrder {
    pub command: CommandMsg,
    pub allow_revert: bool
}

#[cw_serde]
pub enum CommandMsg {

    // Catalyst
    LocalSwap {
        vault: String,
        from_asset_ref: String,
        to_asset_ref: String,
        amount: CoinAmount,
        min_out: Uint128
    },
    SendAsset {
        vault: String,
        channel_id: Bytes32,
        to_vault: Binary,
        to_account: Binary,
        from_asset_ref: String,
        to_asset_index: u8,
        amount: CoinAmount,
        min_out: U256,
        fallback_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary,
        incentive: IncentiveDescription
    },
    SendLiquidity {
        vault: String,
        channel_id: Bytes32,
        to_vault: Binary,
        to_account: Binary,
        amount: ValueAmount,
        min_vault_tokens: U256,
        min_reference_asset: U256,
        fallback_account: String,
        calldata: Binary,
        incentive: IncentiveDescription
    },
    WithdrawAll {
        vault: String,
        amount: ValueAmount,
        min_out: Vec<Uint128>
    },
    WithdrawMixed {
        vault: String,
        amount: ValueAmount,
        withdraw_ratio: Vec<Uint64>,
        min_out: Vec<Uint128>,
    },
    DepositMixed {
        vault: String,
        deposit_amounts: Vec<CoinAmount>,
        min_out: Uint128
    },

    // Payments
    Sweep {
        denoms: Vec<Denom>,
        minimum_amounts: Vec<Uint128>,
        recipient: Account
    },
    SweepAll {
        recipient: Account
    },
    Transfer {
        amounts: Vec<CoinAmount>,
        recipient: Account
    },
    PayPortion {
        denoms: Vec<Denom>,
        bips: Vec<Uint128>,
        recipient: Account
    },
    BalanceCheck {
        denoms: Vec<Denom>,
        minimum_amounts: Vec<Uint128>,
        account: Account
    },

    // Swap Cancel
    AllowCancel {
        authority: String,
        identifier: Binary
    }
}



// Commands Execution
// ************************************************************************************************

/// Return type for the commands execution handlers. It can be either a `CosmosMsg` to be
/// dispatched, or the 'Result' of an atomic check operation.
pub enum CommandResult {
    Message(CosmosMsg),
    Check(Result<(), String>)
}

/// Command executor selector.
/// 
/// # Arguments:
/// * `command` - The command to execute.
/// 
pub fn execute_command(
    deps: &Deps,
    env: &Env,
    command: CommandMsg
) -> Result<CommandResult, ContractError> {

    match command {
        CommandMsg::LocalSwap {
            vault,
            from_asset_ref,
            to_asset_ref,
            amount,
            min_out
        } => catalyst::execute_local_swap(
            deps,
            env,
            vault,
            from_asset_ref,
            to_asset_ref,
            amount,
            min_out
        ),
        CommandMsg::SendAsset {
            vault,
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
        } => catalyst::execute_send_asset(
            deps,
            env,
            vault,
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
        ),
        CommandMsg::SendLiquidity {
            vault,
            channel_id,
            to_vault,
            to_account,
            amount,
            min_vault_tokens,
            min_reference_asset,
            fallback_account,
            calldata,
            incentive
        } => catalyst::execute_send_liquidity(
            deps,
            env,
            vault,
            channel_id,
            to_vault,
            to_account,
            amount,
            min_vault_tokens,
            min_reference_asset,
            fallback_account,
            calldata,
            incentive
        ),
        CommandMsg::WithdrawAll {
            vault,
            amount,
            min_out
        } => catalyst::execute_withdraw_all(
            deps,
            env,
            vault,
            amount,
            min_out
        ),
        CommandMsg::WithdrawMixed {
            vault,
            amount,
            withdraw_ratio,
            min_out
        } => catalyst::execute_withdraw_mixed(
            deps,
            env,
            vault,
            amount,
            withdraw_ratio,
            min_out
        ),
        CommandMsg::DepositMixed {
            vault,
            deposit_amounts,
            min_out
        } => catalyst::execute_deposit_mixed(
            deps,
            env,
            vault,
            deposit_amounts,
            min_out
        ),
        CommandMsg::Sweep {
            denoms,
            minimum_amounts,
            recipient
        } => payments::execute_sweep(
            deps,
            env,
            denoms,
            minimum_amounts,
            recipient
        ),
        CommandMsg::SweepAll {
            recipient
        } => payments::execute_sweep_all(
            deps,
            env,
            recipient
        ),
        CommandMsg::Transfer {
            amounts,
            recipient
        } => payments::execute_transfer(
            deps,
            env,
            amounts,
            recipient
        ),
        CommandMsg::PayPortion {
            denoms,
            bips,
            recipient
        } => payments::execute_pay_portion(
            deps,
            env,
            denoms,
            bips,
            recipient
        ),
        CommandMsg::BalanceCheck {
            denoms,
            minimum_amounts,
            account
        } => payments::execute_balance_check(
            deps,
            env,
            denoms,
            minimum_amounts,
            account
        ),
        CommandMsg::AllowCancel {
            authority,
            identifier
        } => cancel_swap::execute_allow_cancel(
            deps,
            authority,
            identifier
        )
    }

}
