use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Empty, Uint128, Addr, Deps, StdResult, Binary, SubMsg, CosmosMsg, to_json_binary, StdError};
use cw_multi_test::{Module, ContractWrapper, Executor};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use test_helpers::env::CustomApp;

use crate::{msg::ExecuteMsg as RouterExecuteMsg, commands::{CommandOrder, CommandMsg}, executors::types::Account};




// Mock Contract
// ************************************************************************************************

const ROUTER_ADDR: Item<String> = Item::new("malicious-vault-router-address");

#[cw_serde]
pub struct MaliciousVaultInstantiateMsg {
    router: Addr
}

#[cw_serde]
pub enum MaliciousVaultExecuteMsg {
    LocalSwap {
        from_asset_ref: String,
        to_asset_ref: String,
        amount: Uint128,
        min_out: Uint128,
    }
}


pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: MaliciousVaultInstantiateMsg
) -> Result<Response, StdError> {

    ROUTER_ADDR.save(deps.storage, &msg.router.to_string()).unwrap();

    Ok(Response::new())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: MaliciousVaultExecuteMsg,
) -> Result<Response, StdError> {

    match msg {
        MaliciousVaultExecuteMsg::LocalSwap {
            from_asset_ref: _,
            to_asset_ref: _,
            amount: _,
            min_out: _
        } => execute_reentry(deps.as_ref(), env),
    }
}

pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    unimplemented!()
}


fn execute_reentry(
    deps: Deps,
    env: Env
) -> Result<Response, StdError> {

    let router = ROUTER_ADDR.load(deps.storage).unwrap();

    // Get all router denoms
    let router_coins = deps.querier.query_all_balances(router.clone()).unwrap();
    let denoms: Vec<String> = router_coins.into_iter()
        .map(|coin| coin.denom)
        .collect();

    let reentry_msg = RouterExecuteMsg::Execute {
        command_orders: vec![
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: denoms.clone(),
                    minimum_amounts: denoms.iter().map(|_| Uint128::zero()).collect(),
                    recipient: Account::Address(env.contract.address.to_string())
                },
                allow_revert: false
            }
        ],
        deadline: None
    };

    Ok(Response::new()
        .add_submessage(
            SubMsg::new(
                CosmosMsg::Wasm(
                    cosmwasm_std::WasmMsg::Execute {
                        contract_addr: router,
                        msg: to_json_binary(&reentry_msg).unwrap(),
                        funds: vec![]
                    }
                )
            )
        )
    )
}



// Test Helpers ***********************************************************************************

pub fn mock_malicious_vault_storage<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>
) -> u64
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty>,
    ExecC: Clone + Debug + DeserializeOwned + JsonSchema + PartialEq + 'static
{

    // Create contract wrapper
    let contract = ContractWrapper::new_with_empty(
        execute,
        instantiate,
        query,
    );

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}


pub fn mock_instantiate_malicious_vault<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    router: Addr
) -> Addr
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty>,
    ExecC: Clone + Debug + DeserializeOwned + JsonSchema + PartialEq + 'static
{

    let contract_code_storage = mock_malicious_vault_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked("some-account"),
        &MaliciousVaultInstantiateMsg {
            router,
        },
        &[],
        "malicious-vault",
        None
    ).unwrap()
}