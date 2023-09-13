use cosmwasm_std::{Uint64, Uint128, Addr, CosmosMsg, Coin, Binary};
use cw_multi_test::{ContractWrapper, Executor, AppResponse};

use catalyst_vault_common::bindings::native_asset_vault_modules::{NativeAsset, NativeAssetCustomMsg};
use test_helpers::asset::TestNativeAsset;

use test_helpers::definitions::SETUP_MASTER;
use test_helpers::env::CustomTestEnv;
use test_helpers::env::env_native_asset::TestNativeAssetEnv;
use test_helpers::misc::encode_payload_address;
use test_helpers::contract::{mock_factory_deploy_vault, mock_set_vault_connection, mock_instantiate_interface};

use crate::commands::CommandResult;



// Definition
// ********************************************************************************************

pub const ROUTER: &str = "router";



// Helpers
// ********************************************************************************************

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

    pub fn run_executor_result(
        &self,
        test_env: &mut TestNativeAssetEnv,
        router: Addr,
        command_result: CommandResult,
        fund_router: Option<Vec<Coin>>
    ) -> AppResponse {

        if let Some(router_coins) = fund_router {
            test_env.get_app().send_tokens(
                Addr::unchecked(SETUP_MASTER),
                router.clone(),
                &router_coins
            ).unwrap();
        }

        match command_result {
            CommandResult::Message(msg) => {

                let casted_msg: CosmosMsg<NativeAssetCustomMsg> = match msg {
                    CosmosMsg::Wasm(wasm_msg) => CosmosMsg::<NativeAssetCustomMsg>::Wasm(wasm_msg),
                    _ => panic!("Unexpected cosmos message type."),
                };

                test_env.get_app().execute(
                    router,
                    casted_msg
                ).unwrap()

            },
            CommandResult::Check(_) => panic!("Invalid 'check' CommandResult (expecting message)"),
        }
    }

}