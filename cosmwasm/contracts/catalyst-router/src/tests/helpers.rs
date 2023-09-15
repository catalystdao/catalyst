use cosmwasm_std::{Uint64, Uint128, Addr, CosmosMsg, Coin, Binary, Empty};
use cw_multi_test::{ContractWrapper, Executor, AppResponse, Module};

use catalyst_vault_common::bindings::native_asset_vault_modules::{NativeAsset, NativeAssetCustomMsg};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use test_helpers::asset::TestNativeAsset;
use test_helpers::definitions::SETUP_MASTER;
use test_helpers::env::{CustomTestEnv, CustomApp};
use test_helpers::env::env_native_asset::TestNativeAssetEnv;
use test_helpers::misc::encode_payload_address;
use test_helpers::contract::{mock_factory_deploy_vault, mock_set_vault_connection, mock_instantiate_interface};

use crate::commands::CommandResult;



// Definition
// ********************************************************************************************

pub const ROUTER    : &str = "router";
pub const RECIPIENT : &str = "recipient";



// Helpers
// ********************************************************************************************

pub fn router_contract_storage<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>
) -> u64
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty>,
    ExecC: Clone + Debug + DeserializeOwned + JsonSchema + PartialEq + 'static
{

    // Create contract wrapper
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    ).with_reply_empty(crate::contract::reply);

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}


pub fn mock_instantiate_router<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>
) -> Addr
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty>,
    ExecC: Clone + Debug + DeserializeOwned + JsonSchema + PartialEq + 'static
{

    let contract_code_storage = router_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &crate::msg::InstantiateMsg {},
        &[],
        "router",
        None
    ).unwrap()
}



pub fn fund_account(
    test_env: &mut TestNativeAssetEnv,
    account: Addr,
    funds: Vec<Coin>
) {
    let funds: Vec<Coin> = funds
        .into_iter()
        .filter(|coin| !coin.amount.is_zero())
        .collect();

    test_env.get_app().send_tokens(
        Addr::unchecked(SETUP_MASTER),
        account,
        &funds
    ).unwrap();
}


pub fn run_command_result(
    test_env: &mut TestNativeAssetEnv,
    router: Addr,
    command_result: CommandResult
) -> AppResponse {

    match command_result {
        CommandResult::Message(msg) => {

            let casted_msg: CosmosMsg<NativeAssetCustomMsg> = match msg {
                CosmosMsg::Wasm(wasm_msg) => CosmosMsg::<NativeAssetCustomMsg>::Wasm(wasm_msg),
                CosmosMsg::Bank(bank_msg) => CosmosMsg::<NativeAssetCustomMsg>::Bank(bank_msg),
                _ => panic!("Unexpected cosmos message type."),
            };

            test_env.get_app().execute(
                router,
                casted_msg
            ).unwrap()

        },
        CommandResult::Check(check_result) => {
            match check_result {
                Ok(_) => AppResponse::default(),
                Err(error) => panic!("Command result check error: {}", error),
            }
        },
    }
}



pub struct  MockVault {
    pub vault_assets: Vec<TestNativeAsset>,
    pub vault: Addr,
    pub target_vault: Binary,
    pub channel_id: String
}

impl MockVault {

    pub fn new(test_env: &mut TestNativeAssetEnv) -> Self {
        
        // 'Deploy' the vault contract
        let vault_code_id = test_env
            .get_app()
            .store_code(Box::new(
                ContractWrapper::new(
                    catalyst_vault_volatile::contract::execute,
                    catalyst_vault_volatile::contract::instantiate,
                    catalyst_vault_volatile::contract::query,
                )
            ));

        let interface = mock_instantiate_interface(test_env.get_app());
        let vault_assets = test_env.get_assets()[..2].to_vec();
        let vault_initial_balances = vec![
            Uint128::new(100000u128),
            Uint128::new(200000u128)
        ];
        let vault_weights = [Uint128::one(), Uint128::one()].to_vec();

        let vault = mock_factory_deploy_vault::<NativeAsset, _, _>(
            test_env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            Uint64::new(1000000000000000000u64),
            vault_code_id,
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        let channel_id = "channel_id".to_string();
        mock_set_vault_connection(
            test_env.get_app(),
            vault.clone(),
            channel_id.clone(),
            target_vault.clone(),
            true
        );

        MockVault {
            vault_assets,
            vault,
            target_vault,
            channel_id
        }

    }

}