use catalyst_types::U256;
use catalyst_vault_common::msg::BalanceResponse;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Env, Binary, CosmosMsg, to_binary, from_binary, Uint128, Uint64, Coin, Deps};

use crate::{commands::CommandResult, error::ContractError, executors::types::{Amount, CoinAmount}};

type CatalystExecuteMsg = catalyst_vault_common::msg::ExecuteMsg<()>;

#[cw_serde]
pub struct LocalSwapCommand {
    pub vault: String,
    pub from_asset_ref: String,
    pub to_asset_ref: String,
    pub amount: CoinAmount,
    pub min_out: Uint128
}


#[cw_serde]
pub struct SendAssetCommand {
    pub vault: String,
    pub channel_id: String,
    pub to_vault: Binary,
    pub to_account: Binary,
    pub from_asset_ref: String,
    pub to_asset_index: u8,
    pub amount: CoinAmount,
    pub min_out: U256,
    pub fallback_account: String,
    pub calldata: Binary
}


#[cw_serde]
pub struct SendLiquidityCommand {
    pub vault: String,
    pub channel_id: String,
    pub to_vault: Binary,
    pub to_account: Binary,
    pub amount: Amount,
    pub min_vault_tokens: U256,
    pub min_reference_asset: U256,
    pub fallback_account: String,
    pub calldata: Binary
}


#[cw_serde]
pub struct DepositMixedCommand {
    pub vault: String,
    pub deposit_amounts: Vec<CoinAmount>,
    pub min_out: Uint128
}


#[cw_serde]
pub struct WithdrawAllCommand {
    pub vault: String,
    pub amount: Amount,
    pub min_out: Vec<Uint128>
}


#[cw_serde]
pub struct WithdrawMixedCommand {
    pub vault: String,
    pub amount: Amount,
    pub withdraw_ratio: Vec<Uint64>,
    pub min_out: Vec<Uint128>,
}


#[cw_serde]
struct BalanceQuery {
    address: String
}


pub fn execute_local_swap(
    deps: &Deps,
    env: &Env,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<LocalSwapCommand>(input)?;

    let swap_amount = args.amount.get_amount(deps, env)?;
    
    let msg = CatalystExecuteMsg::LocalSwap {
        from_asset_ref: args.from_asset_ref,
        to_asset_ref: args.to_asset_ref,
        amount: swap_amount.amount,
        min_out: args.min_out
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: args.vault,
                msg: to_binary(&msg)?,
                funds: vec![swap_amount]
            }
        )
    ))
}


pub fn execute_send_asset(
    deps: &Deps,
    env: &Env,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<SendAssetCommand>(input)?;

    let swap_amount = args.amount.get_amount(deps, env)?;
    
    let msg = CatalystExecuteMsg::SendAsset {
        channel_id: args.channel_id,
        to_vault: args.to_vault,
        to_account: args.to_account,
        from_asset_ref: args.from_asset_ref,
        to_asset_index: args.to_asset_index,
        amount: swap_amount.amount,
        min_out: args.min_out,
        fallback_account: args.fallback_account,
        calldata: args.calldata
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: args.vault,
                msg: to_binary(&msg)?,
                funds: vec![swap_amount]
            }
        )
    ))
}


pub fn execute_send_liquidity(
    deps: &Deps,
    env: &Env,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<SendLiquidityCommand>(input)?;

    let send_amount = match args.amount {
        Amount::Amount(amount) => amount,
        Amount::RouterBalance() => deps.querier.query_wasm_smart::<BalanceResponse>(
            args.vault.clone(),
            &BalanceQuery{ address: env.contract.address.to_string() }
        )?.balance,
    };
    
    let msg = CatalystExecuteMsg::SendLiquidity {
        channel_id: args.channel_id,
        to_vault: args.to_vault,
        to_account: args.to_account,
        amount: send_amount,
        min_vault_tokens: args.min_vault_tokens,
        min_reference_asset: args.min_reference_asset,
        fallback_account: args.fallback_account,
        calldata: args.calldata
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: args.vault,
                msg: to_binary(&msg)?,
                funds: vec![]
            }
        )
    ))
}


pub fn execute_deposit_mixed(
    deps: &Deps,
    env: &Env,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<DepositMixedCommand>(input)?;

    let deposit_coins = args.deposit_amounts.iter()
        .map(|amount| amount.get_amount(deps, env))
        .collect::<Result<Vec<Coin>, _>>()?;

    let deposit_amounts = deposit_coins.iter()
        .filter_map(|coin| {
            match coin.amount.is_zero() {
                true => None,
                false => Some(coin.amount),
            }
        })
        .collect();

    let msg = CatalystExecuteMsg::DepositMixed {
        deposit_amounts,
        min_out: args.min_out
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: args.vault,
                msg: to_binary(&msg)?,
                funds: deposit_coins
            }
        )
    ))
}


pub fn execute_withdraw_equal(
    deps: &Deps,
    env: &Env,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<WithdrawAllCommand>(input)?;

    let withdraw_amount = match args.amount {
        Amount::Amount(amount) => amount,
        Amount::RouterBalance() => deps.querier.query_wasm_smart::<BalanceResponse>(
            args.vault.clone(),
            &BalanceQuery{ address: env.contract.address.to_string() }
        )?.balance,
    };

    let msg = CatalystExecuteMsg::WithdrawAll {
        vault_tokens: withdraw_amount,
        min_out: args.min_out
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: args.vault,
                msg: to_binary(&msg)?,
                funds: vec![]
            }
        )
    ))
}


pub fn execute_withdraw_mixed(
    deps: &Deps,
    env: &Env,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<WithdrawMixedCommand>(input)?;

    let withdraw_amount = match args.amount {
        Amount::Amount(amount) => amount,
        Amount::RouterBalance() => deps.querier.query_wasm_smart::<BalanceResponse>(
            args.vault.clone(),
            &BalanceQuery{ address: env.contract.address.to_string() }
        )?.balance,
    };

    let msg = CatalystExecuteMsg::WithdrawMixed {
        vault_tokens: withdraw_amount,
        withdraw_ratio: args.withdraw_ratio,
        min_out: args.min_out,
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: args.vault,
                msg: to_binary(&msg)?,
                funds: vec![]
            }
        )
    ))
}

