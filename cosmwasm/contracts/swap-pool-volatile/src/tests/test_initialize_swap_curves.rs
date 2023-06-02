mod test_volatile_initialize_swap_curves {

    use cosmwasm_std::{Uint128, Addr};
    use cw20::{ TokenInfoResponse, BalanceResponse, Cw20QueryMsg};
    use cw_multi_test::{App, Executor};
    use ethnum::U256;
    use fixed_point_math::LN2;
    use swap_pool_common::{ContractError, msg::{AssetsResponse, WeightsResponse, GetLimitCapacityResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse}};

    use crate::{tests::helpers::{mock_instantiate_vault, DEPOSITOR, DEPLOYER, InitializeSwapCurvesMockConfig, deploy_test_tokens, mock_test_token_definitions, WAD, SETUP_MASTER}, msg::VolatileExecuteMsg};



    #[test]
    fn test_initialize() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::from(3u64) * WAD
            ],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
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
        let weights: Vec<u64> = app
            .wrap()
            .query_wasm_smart::<WeightsResponse>(vault.clone(), &crate::msg::QueryMsg::Weights {})
            .unwrap()
            .weights;

        assert_eq!(
            weights,
            initialize_msg.weights
        );

        // Query and verify the security limit
        let max_limit_capacity: U256 = app
            .wrap()
            .query_wasm_smart::<GetLimitCapacityResponse>(vault.clone(), &crate::msg::QueryMsg::GetLimitCapacity {})
            .unwrap()
            .capacity;

        assert_eq!(
            max_limit_capacity,
            U256::from(weights.iter().sum::<u64>()) * LN2
        );

        // Query and verify the pool token supply
        let pool_token_supply: Uint128 = app
            .wrap()
            .query_wasm_smart::<TokenInfoResponse>(vault.clone(), &crate::msg::QueryMsg::TokenInfo {})
            .unwrap()
            .total_supply;

        assert_eq!(
            pool_token_supply,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify the pool tokens of the depositor
        let depositor_pool_tokens: Uint128 = app
            .wrap()
            .query_wasm_smart::<BalanceResponse>(vault.clone(), &crate::msg::QueryMsg::Balance { address: DEPOSITOR.to_string() })
            .unwrap()
            .balance;

        assert_eq!(
            depositor_pool_tokens,
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
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::from(3u64) * WAD
            ],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );

        // Initialize swap curves
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        ).unwrap();



        // Tested action: initialize swap curves twice
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
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
        let vault = mock_instantiate_vault(&mut app, None);

        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: vec![],
            assets_balances: vec![],
            weights: vec![],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER.to_string()
        };


        // Tested action: initialize swap curves without assets
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));

    }


    #[test]
    fn test_too_many_assets() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens_definitions = mock_test_token_definitions(4);    // ! Generate 4 tokens definitions
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            Some(test_tokens_definitions)
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::from(3u64) * WAD,
                Uint128::from(4u64) * WAD
            ],
            weights: vec![1u64, 1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with too many assets
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));

    }


    #[test]
    fn test_invalid_asset_balances_count() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD       // ! Only 2 asset balances are specified
            ],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid asset balance count
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_zero_asset_balance() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::zero()                 // ! Asset balance is set to 0
            ],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an asset balance set to 0
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_invalid_weights_count() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::from(3u64) * WAD
            ],
            weights: vec![1u64, 1u64],    // ! Only 2 weights are specified
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid weights count
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_zero_weight() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::from(3u64) * WAD
            ],
            weights: vec![1u64, 1u64, 0u64],    // ! Weight set to 0
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with a weight set to 0
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );



        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_invalid_amp() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        // Create tokens and set vault allowances
        let test_tokens = deploy_test_tokens(
            &mut app,
            None,
            None
        );

        // Define InitializeSwapCurves parameters
        let initialize_msg = InitializeSwapCurvesMockConfig {
            assets: test_tokens.iter().map(|addr| addr.to_string()).collect(),
            assets_balances: vec![
                Uint128::from(1u64) * WAD,
                Uint128::from(2u64) * WAD,
                Uint128::from(3u64) * WAD
            ],
            weights: vec![1u64, 1u64, 1u64],
            amp: 900000000000000000u64,                 // ! Invalid amplification is specified
            depositor: DEPOSITOR.to_string()
        };

        // Transfer tokens to the vault
        initialize_msg.transfer_vault_allowances(
            &mut app,
            vault.to_string(),
            Addr::unchecked(SETUP_MASTER)
        );



        // Tested action: initialize swap curves with an invalid amplification value
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &initialize_msg.clone().into(),
            &[]
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAmplification {}
        ));
        
    }


}