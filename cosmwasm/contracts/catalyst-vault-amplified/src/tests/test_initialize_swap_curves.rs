mod test_amplified_initialize_swap_curves {

    use std::ops::Div;
    use cosmwasm_std::{Uint128, Addr, Uint64, Attribute};
    use cw_multi_test::Executor;
    use catalyst_types::{U256, u256};
    use fixed_point_math::WAD;
    use catalyst_vault_common::{ContractError, msg::{AssetsResponse, WeightResponse, GetLimitCapacityResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, AssetResponse}, state::INITIAL_MINT_AMOUNT, event::format_vec_for_event, bindings::{Asset, AssetTrait}};
    use test_helpers::{definitions::{SETUP_MASTER, DEPOSITOR, DEPLOYER, VAULT_TOKEN_DENOM}, contract::{mock_instantiate_vault, InitializeSwapCurvesMockConfig, mock_instantiate_vault_msg}, env::CustomTestEnv, asset::CustomTestAsset, vault_token::CustomTestVaultToken};

    use crate::tests::{TestEnv, TestAsset, TestApp, TestVaultToken};
    use crate::{tests::{helpers::amplified_vault_contract_storage, parameters::{AMPLIFICATION, TEST_VAULT_WEIGHTS, TEST_VAULT_BALANCES, TEST_VAULT_ASSET_COUNT}}, msg::AmplificationResponse};


    #[test]
    fn test_initialize() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.clone(),
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        ).unwrap();



        // Verify the assets have been transferred to the vault
        test_tokens
            .iter()
            .zip(&initialize_msg.assets_balances)
            .for_each(|(asset, balance)| {
                let queried_balance: Uint128 = asset.query_balance(env.get_app(), vault.to_string());

                assert_eq!(
                    queried_balance,
                    balance
                );
            });

        // Query and verify the assets
        let assets = env.get_app()
            .wrap()
            .query_wasm_smart::<AssetsResponse<Asset>>(vault.clone(), &crate::msg::QueryMsg::Assets {})
            .unwrap()
            .assets;

        assert_eq!(
            assets,
            initialize_msg.assets
                .iter()
                .map(|asset| asset.clone().into())
                .collect::<Vec<_>>()
        );

        // Query and verify the individual assets
        assets
            .iter()
            .for_each(|asset| {
                let queried_asset = env.get_app()
                    .wrap()
                    .query_wasm_smart::<AssetResponse<Asset>>(
                        vault.clone(),
                        &crate::msg::QueryMsg::Asset {
                            asset_ref: asset.get_asset_ref().to_owned()
                        }
                    )
                    .unwrap()
                    .asset;

                assert_eq!(
                    queried_asset,
                    asset.clone()
                )
            });

        // Query and verify the weights
        assets
            .iter()
            .zip(&initialize_msg.weights)
            .for_each(|(asset, weight)| {

                let queried_weight: Uint128 = env.get_app()
                    .wrap()
                    .query_wasm_smart::<WeightResponse>(
                        vault.clone(),
                        &crate::msg::QueryMsg::Weight { asset_ref: asset.get_asset_ref().to_string() }
                    )
                    .unwrap()
                    .weight;

                assert_eq!(
                    weight,
                    queried_weight
                );
            });



        // Query and verify the security limit
        let max_limit_capacity: U256 = env.get_app()
            .wrap()
            .query_wasm_smart::<GetLimitCapacityResponse>(vault.clone(), &crate::msg::QueryMsg::GetLimitCapacity {})
            .unwrap()
            .capacity;

        let expected_max_limit_capacity: U256 = initialize_msg.assets_balances.iter()
            .zip(initialize_msg.weights)
            .fold(U256::zero(), |acc, (balance, weight)| -> U256 {
                acc.checked_add(
                    U256::from(*balance).checked_mul(U256::from(weight)).unwrap()
                ).unwrap()
            })
            .div(u256!("2"));

        assert_eq!(
            max_limit_capacity,
            expected_max_limit_capacity
        );

        // Query and verify the vault token supply
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let vault_token_supply = vault_token.total_supply(env.get_app());

        assert_eq!(
            vault_token_supply,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify the vault tokens of the depositor
        let depositor_vault_tokens: Uint128 = vault_token.query_balance(env.get_app(), DEPOSITOR.to_string());

        assert_eq!(
            depositor_vault_tokens,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify escrow totals are intialized
        initialize_msg.assets
            .iter()
            .for_each(|asset| {
                let total_escrowed_balance: Uint128 = env.get_app()
                    .wrap()
                    .query_wasm_smart::<TotalEscrowedAssetResponse>(
                        vault.clone(),
                        &crate::msg::QueryMsg::TotalEscrowedAsset { asset_ref: asset.get_asset_ref() })
                    .unwrap()
                    .amount;

                assert_eq!(
                    total_escrowed_balance,
                    Uint128::zero()
                );
            });

        let total_escrowed_liquidity: Uint128 = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(vault.clone(), &crate::msg::QueryMsg::TotalEscrowedLiquidity {})
            .unwrap()
            .amount;

        assert_eq!(
            total_escrowed_liquidity,
            Uint128::zero()
        );



        // Query and verify the amplification
        let amplification: Uint64 = env.get_app()
            .wrap()
            .query_wasm_smart::<AmplificationResponse>(vault.clone(), &crate::msg::QueryMsg::Amplification {})
            .unwrap()
            .amplification;

        assert_eq!(
            amplification,
            initialize_msg.amp
        );

    }


    #[test]
    fn test_initialize_deposit_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves
        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-deposit");
        
        assert_eq!(
            event.attributes[1],
            Attribute::new("to_account", DEPOSITOR)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("mint", INITIAL_MINT_AMOUNT.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("deposit_amounts", format_vec_for_event(TEST_VAULT_BALANCES.to_vec()))
        );

    }


    #[test]
    fn test_initialize_twice() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );

        // Initialize swap curves
        let _response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        ).unwrap();



        // Tested action: initialize swap curves twice
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );



        // Make sure second initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


    #[test]
    fn test_no_assets() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        let initialize_msg = InitializeSwapCurvesMockConfig::<Asset, TestAsset, TestApp> {
            assets: vec![],
            assets_balances: vec![],
            weights: vec![],
            amp: AMPLIFICATION,
            depositor: DEPLOYER.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };


        // Tested action: initialize swap curves without assets
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAssets {}
        ));

    }


    #[test]
    fn test_too_many_assets() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..4].to_vec();   // ! Generate 4 tokens definitions

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::from(3u64) * WAD.as_uint128(),
                Uint128::from(4u64) * WAD.as_uint128()
            ],
            weights: vec![Uint128::one(), Uint128::one(), Uint128::one(), Uint128::one()],
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with too many assets
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAssets {}
        ));

    }


    #[test]
    fn test_zero_asset_balance() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..3].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::zero()                 // ! Asset balance is set to 0
            ],
            weights: vec![Uint128::one(), Uint128::one(), Uint128::one()],
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an asset balance set to 0
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidZeroBalance {}
        ));
        
    }


    #[test]
    fn test_invalid_weights_count() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..3].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::from(3u64) * WAD.as_uint128()
            ],
            weights: vec![Uint128::one(), Uint128::one()],    // ! Only 2 weights are specified
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid weights count
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidParameters { reason: err_reason }
                if err_reason == "Invalid weights count.".to_string()
        ));
        
    }


    #[test]
    fn test_zero_weight() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..3].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::from(3u64) * WAD.as_uint128()
            ],
            weights: vec![Uint128::one(), Uint128::one(), Uint128::zero()],    // ! Weight set to 0
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with a weight set to 0
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidWeight {}
        ));
        
    }


    #[test]
    fn test_invalid_amp() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: Uint64::new(1000000000000000000u64),                 // ! Invalid amplification is specified
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid amplification value
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAmplification {}
        ));
        
    }


    #[test]
    fn test_initializer_must_be_instantiator() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        
        let instantiate_msg = mock_instantiate_vault_msg(None);

        let vault = env.get_app().instantiate_contract(
            vault_code_id,
            Addr::unchecked(SETUP_MASTER),
            &instantiate_msg,
            &[],
            "vault",
            None
        ).unwrap();

        // Create tokens and set vault allowances
        let test_tokens = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens,
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string(),
            phantom_data: std::marker::PhantomData::<(Asset, _)>
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_assets(
            env.get_app(),
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an unauthorized caller
        let response_result = env.execute_contract(
            Addr::unchecked("not-setup-master"),    // ! Not the vault instantiator
            vault.clone(),
            &initialize_msg.build_execute_msg(),
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}