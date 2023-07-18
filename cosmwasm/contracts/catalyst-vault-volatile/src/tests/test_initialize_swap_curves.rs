mod test_volatile_initialize_swap_curves {

    use cosmwasm_std::{Uint128, Addr, Uint64};
    use cw20::{ TokenInfoResponse, BalanceResponse, Cw20QueryMsg};
    use cw_multi_test::{App, Executor};
    use catalyst_types::U256;
    use fixed_point_math::{LN2, WAD};
    use catalyst_vault_common::{ContractError, msg::{AssetsResponse, WeightResponse, GetLimitCapacityResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse}};
    use test_helpers::{token::deploy_test_tokens, definitions::{SETUP_MASTER, DEPOSITOR, DEPLOYER}, contract::{mock_instantiate_vault, InitializeSwapCurvesMockConfig}};

    use crate::tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}};



    #[test]
    fn test_initialize() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            TEST_VAULT_ASSET_COUNT
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves
        let _response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        ).unwrap();



    //     //TODO check response attributes (event)

    //     //TODO verify only the factory can invoke initialize_swap_curves

        // Verify the assets have been transferred to the vault
        test_tokens
            .iter()
            .zip(&initialize_msg.assets_balances)
            .for_each(|(asset, balance)| {
                let queried_balance: Uint128 = app.wrap().query_wasm_smart::<BalanceResponse>(
                    asset,
                    &Cw20QueryMsg::Balance { address: vault.to_string() }
                ).unwrap().balance;

                assert_eq!(
                    queried_balance,
                    balance
                );
            });

        // Query and verify the assets
        let assets: Vec<Addr> = app
            .wrap()
            .query_wasm_smart::<AssetsResponse>(vault.clone(), &crate::msg::QueryMsg::Assets {})
            .unwrap()
            .assets;

        assert_eq!(
            assets,
            initialize_msg.assets
                .iter()
                .map(|asset| Addr::unchecked(asset))
                .collect::<Vec<Addr>>()
        );

        // Query and verify the weights
        assets
            .iter()
            .zip(&initialize_msg.weights)
            .for_each(|(asset, weight)| {

                let queried_weight: Uint128 = app
                    .wrap()
                    .query_wasm_smart::<WeightResponse>(
                        vault.clone(),
                        &crate::msg::QueryMsg::Weight { asset: asset.to_string() }
                    )
                    .unwrap()
                    .weight;

                assert_eq!(
                    weight,
                    queried_weight
                );
            });



        // Query and verify the security limit
        let max_limit_capacity: U256 = app
            .wrap()
            .query_wasm_smart::<GetLimitCapacityResponse>(vault.clone(), &crate::msg::QueryMsg::GetLimitCapacity {})
            .unwrap()
            .capacity;

        assert_eq!(
            max_limit_capacity,
            U256::from(initialize_msg.weights.iter().sum::<Uint128>()) * LN2
        );

        // Query and verify the vault token supply
        let vault_token_supply: Uint128 = app
            .wrap()
            .query_wasm_smart::<TokenInfoResponse>(vault.clone(), &crate::msg::QueryMsg::TokenInfo {})
            .unwrap()
            .total_supply;

        assert_eq!(
            vault_token_supply,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify the vault tokens of the depositor
        let depositor_vault_tokens: Uint128 = app
            .wrap()
            .query_wasm_smart::<BalanceResponse>(vault.clone(), &crate::msg::QueryMsg::Balance { address: DEPOSITOR.to_string() })
            .unwrap()
            .balance;

        assert_eq!(
            depositor_vault_tokens,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify escrow totals are intialized
        initialize_msg.assets
            .iter()
            .for_each(|asset| {
                let total_escrowed_balance: Uint128 = app
                    .wrap()
                    .query_wasm_smart::<TotalEscrowedAssetResponse>(
                        vault.clone(),
                        &crate::msg::QueryMsg::TotalEscrowedAsset { asset: asset.to_string() })
                    .unwrap()
                    .amount;

                assert_eq!(
                    total_escrowed_balance,
                    Uint128::zero()
                );
            });

        let total_escrowed_liquidity: Uint128 = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(vault.clone(), &crate::msg::QueryMsg::TotalEscrowedLiquidity {})
            .unwrap()
            .amount;

        assert_eq!(
            total_escrowed_liquidity,
            Uint128::zero()
        );

    }


    #[test]
    fn test_initialize_twice() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            TEST_VAULT_ASSET_COUNT
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );

        // Initialize swap curves
        let _response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        ).unwrap();



        // Tested action: initialize swap curves twice
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        );



        // Make sure second initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


    #[test]
    fn test_no_assets() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: vec![],
            assets_balances: vec![],
            weights: vec![],
            amp: AMPLIFICATION,
            depositor: DEPLOYER.to_string()
        };


        // Tested action: initialize swap curves without assets
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAssets {}
        ));

    }


    #[test]
    fn test_too_many_assets() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            4    // ! Generate 4 tokens definitions
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::from(3u64) * WAD.as_uint128(),
                Uint128::from(4u64) * WAD.as_uint128()
            ],
            weights: vec![Uint128::one(), Uint128::one(), Uint128::one(), Uint128::one()],
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with too many assets
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAssets {}
        ));

    }


    #[test]
    fn test_zero_asset_balance() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            3
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::zero()                 // ! Asset balance is set to 0
            ],
            weights: vec![Uint128::one(), Uint128::one(), Uint128::one()],
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an asset balance set to 0
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidZeroBalance {}
        ));
        
    }


    #[test]
    fn test_invalid_weights_count() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            3
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::from(3u64) * WAD.as_uint128()
            ],
            weights: vec![Uint128::one(), Uint128::one()],    // ! Only 2 weights are specified
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid weights count
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
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

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            3
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD.as_uint128(),
                Uint128::from(2u64) * WAD.as_uint128(),
                Uint128::from(3u64) * WAD.as_uint128()
            ],
            weights: vec![Uint128::one(), Uint128::one(), Uint128::zero()],    // ! Weight set to 0
            amp: AMPLIFICATION,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with a weight set to 0
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidWeight {}
        ));
        
    }


    #[test]
    fn test_invalid_amp() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            SETUP_MASTER.to_string(),
            None,
            TEST_VAULT_ASSET_COUNT
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: TEST_VAULT_BALANCES.to_vec(),
            weights: TEST_VAULT_WEIGHTS.to_vec(),
            amp: Uint64::new(900000000000000000u64),                 // ! Invalid amplification is specified
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid amplification value
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into_execute_msg(),
            &[]
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAmplification {}
        ));
        
    }


}