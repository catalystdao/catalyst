use catalyst_types::{U256, Bytes32};
use catalyst_vault_common::msg::BalanceResponse;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Env, Binary, CosmosMsg, to_binary, Uint128, Uint64, Coin, Deps};

use crate::{commands::CommandResult, error::ContractError, executors::types::{ValueAmount, CoinAmount}};

type CatalystExecuteMsg = catalyst_vault_common::msg::ExecuteMsg<()>;




// Helpers and Definitions
// ************************************************************************************************

#[cw_serde]
enum VaultQuery {
    Balance {
        address: String
    }
}


pub(crate) fn get_vault_token_amount(
    deps: &Deps,
    env: &Env,
    vault: String,
    amount: ValueAmount
) -> Result<Uint128, ContractError> {

    let amount = match amount {
        ValueAmount::Value(amount) => amount,
        ValueAmount::RouterBalance => deps.querier.query_wasm_smart::<BalanceResponse>(
            vault,
            &VaultQuery::Balance{ address: env.contract.address.to_string() }
        )?.balance,
    };

    Ok(amount)
}




// Executors
// ************************************************************************************************

pub fn execute_local_swap(
    deps: &Deps,
    env: &Env,
    vault: String,
    from_asset_ref: String,
    to_asset_ref: String,
    amount: CoinAmount,
    min_out: Uint128
) -> Result<CommandResult, ContractError> {

    let swap_amount = amount.get_coin(deps, env)?;
    
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
    channel_id: Bytes32,
    to_vault: Binary,
    to_account: Binary,
    from_asset_ref: String,
    to_asset_index: u8,
    amount: CoinAmount,
    min_out: U256,
    fallback_account: String,
    calldata: Binary
) -> Result<CommandResult, ContractError> {

    let swap_amount = amount.get_coin(deps, env)?;
    
    let msg = CatalystExecuteMsg::SendAsset {
        channel_id,
        to_vault,
        to_account,
        from_asset_ref,
        to_asset_index,
        amount: swap_amount.amount,
        min_out,
        fallback_account,
        underwrite_incentive_x16: 0u16, //TODO implement in router
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
    channel_id: Bytes32,
    to_vault: Binary,
    to_account: Binary,
    amount: ValueAmount,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    fallback_account: String,
    calldata: Binary
) -> Result<CommandResult, ContractError> {

    let send_amount = get_vault_token_amount(
        deps,
        env,
        vault.clone(),
        amount
    )?;
    
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

    let deposit_coins = deposit_amounts.into_iter()
        .map(|amount| amount.get_coin(deps, env))
        .collect::<Result<Vec<Coin>, _>>()?;

    let deposit_amounts = deposit_coins.iter()
        .map(|coin| coin.amount)
        .collect();

    let send_coins = deposit_coins.into_iter()
        .filter(|coin| !coin.amount.is_zero())
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
                funds: send_coins
            }
        )
    ))
}


pub fn execute_withdraw_all(
    deps: &Deps,
    env: &Env,
    vault: String,
    amount: ValueAmount,
    min_out: Vec<Uint128>
) -> Result<CommandResult, ContractError> {

    let withdraw_amount = get_vault_token_amount(
        deps,
        env,
        vault.clone(),
        amount
    )?;

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
    amount: ValueAmount,
    withdraw_ratio: Vec<Uint64>,
    min_out: Vec<Uint128>,
) -> Result<CommandResult, ContractError> {

    let withdraw_amount = get_vault_token_amount(
        deps,
        env,
        vault.clone(),
        amount
    )?;

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

