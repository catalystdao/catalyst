use catalyst_types::U256;
use catalyst_vault_common::msg::BalanceResponse;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Env, Binary, CosmosMsg, to_binary, Uint128, Uint64, Coin, Deps};

use crate::{commands::CommandResult, error::ContractError, executors::types::{Amount, CoinAmount}};

type CatalystExecuteMsg = catalyst_vault_common::msg::ExecuteMsg<()>;



#[cw_serde]
struct BalanceQuery {
    address: String
}



pub fn execute_local_swap(
    deps: &Deps,
    env: &Env,
    vault: String,
    from_asset_ref: String,
    to_asset_ref: String,
    amount: CoinAmount,
    min_out: Uint128
) -> Result<CommandResult, ContractError> {

    let swap_amount = amount.get_amount(deps, env)?;
    
    let msg = CatalystExecuteMsg::LocalSwap {
        from_asset_ref,
        to_asset_ref,
        amount: swap_amount.amount,
        min_out
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: vault,
                msg: to_binary(&msg)?,
                funds: vec![swap_amount]
            }
        )
    ))
}


pub fn execute_send_asset(
    deps: &Deps,
    env: &Env,
    vault: String,
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    from_asset_ref: String,
    to_asset_index: u8,
    amount: CoinAmount,
    min_out: U256,
    fallback_account: String,
    calldata: Binary
) -> Result<CommandResult, ContractError> {

    let swap_amount = amount.get_amount(deps, env)?;
    
    let msg = CatalystExecuteMsg::SendAsset {
        channel_id,
        to_vault,
        to_account,
        from_asset_ref,
        to_asset_index,
        amount: swap_amount.amount,
        min_out,
        fallback_account,
        calldata
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: vault,
                msg: to_binary(&msg)?,
                funds: vec![swap_amount]
            }
        )
    ))
}


pub fn execute_send_liquidity(
    deps: &Deps,
    env: &Env,
    vault: String,
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    amount: Amount,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    fallback_account: String,
    calldata: Binary
) -> Result<CommandResult, ContractError> {

    let send_amount = match amount {
        Amount::Amount(amount) => amount,
        Amount::RouterBalance() => deps.querier.query_wasm_smart::<BalanceResponse>(
            vault.clone(),
            &BalanceQuery{ address: env.contract.address.to_string() }
        )?.balance,
    };
    
    let msg = CatalystExecuteMsg::SendLiquidity {
        channel_id,
        to_vault,
        to_account,
        amount: send_amount,
        min_vault_tokens,
        min_reference_asset,
        fallback_account,
        calldata
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: vault,
                msg: to_binary(&msg)?,
                funds: vec![]
            }
        )
    ))
}


pub fn execute_deposit_mixed(
    deps: &Deps,
    env: &Env,
    vault: String,
    deposit_amounts: Vec<CoinAmount>,
    min_out: Uint128
) -> Result<CommandResult, ContractError> {

    let deposit_coins = deposit_amounts.iter()
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
        min_out
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: vault,
                msg: to_binary(&msg)?,
                funds: deposit_coins
            }
        )
    ))
}


pub fn execute_withdraw_all(
    deps: &Deps,
    env: &Env,
    vault: String,
    amount: Amount,
    min_out: Vec<Uint128>
) -> Result<CommandResult, ContractError> {

    let withdraw_amount = match amount {
        Amount::Amount(amount) => amount,
        Amount::RouterBalance() => deps.querier.query_wasm_smart::<BalanceResponse>(
            vault.clone(),
            &BalanceQuery{ address: env.contract.address.to_string() }
        )?.balance,
    };

    let msg = CatalystExecuteMsg::WithdrawAll {
        vault_tokens: withdraw_amount,
        min_out
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: vault,
                msg: to_binary(&msg)?,
                funds: vec![]
            }
        )
    ))
}


pub fn execute_withdraw_mixed(
    deps: &Deps,
    env: &Env,
    vault: String,
    amount: Amount,
    withdraw_ratio: Vec<Uint64>,
    min_out: Vec<Uint128>,
) -> Result<CommandResult, ContractError> {

    let withdraw_amount = match amount {
        Amount::Amount(amount) => amount,
        Amount::RouterBalance() => deps.querier.query_wasm_smart::<BalanceResponse>(
            vault.clone(),
            &BalanceQuery{ address: env.contract.address.to_string() }
        )?.balance,
    };

    let msg = CatalystExecuteMsg::WithdrawMixed {
        vault_tokens: withdraw_amount,
        withdraw_ratio,
        min_out,
    };

    Ok(CommandResult::Message(
        CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: vault,
                msg: to_binary(&msg)?,
                funds: vec![]
            }
        )
    ))
}

