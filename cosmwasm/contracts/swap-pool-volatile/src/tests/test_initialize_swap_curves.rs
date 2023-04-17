mod test_volatile_initialize_swap_curves {

    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Uint128, from_binary, Addr, SubMsg, CosmosMsg, to_binary};
    use cw20::{Cw20ExecuteMsg, TokenInfoResponse, BalanceResponse};
    use ethnum::U256;
    use fixed_point_math_lib::fixed_point_math::LN2;
    use swap_pool_common::ContractError;

    use crate::{tests::helpers::{mock_instantiate, DEPOSITOR_ADDR, DEPLOYER_ADDR, InitializeSwapCurvesMockMsg}, contract::{execute, query}};



    #[test]
    fn test_initialize() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(3000u64)],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPOSITOR_ADDR.to_string()
        };


        // Tested action: initialize swap curves
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.clone().into()
        ).unwrap();


        //TODO check response attributes (event)

        //TODO verify only the factory can invoke initialize_swap_curves

        // Verify the asset transfer messages   //TODO overhaul how assets are transferred to the contract once the factory is developped
        let contract_addr = mock_env().contract.address;
        let expected_transfer_msgs = initialize_msg.assets
            .iter()
            .zip(initialize_msg.assets_balances)
            .map(|(asset, balance)| {
                SubMsg::new(
                    CosmosMsg::Wasm(
                        cosmwasm_std::WasmMsg::Execute {
                            contract_addr: asset.to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                                owner: DEPLOYER_ADDR.to_string(),       // ! From the caller //TODO note this is not the 'DEPOSITOR_ADDR', but the caller of the tx. Review
                                recipient: contract_addr.to_string(),   // ! To the vault
                                amount: balance                         // ! Of amount 'balance'
                            }).unwrap(),
                            funds: vec![]
                        }
                    )
                )
            })
            .collect::<Vec<SubMsg>>();

        assert_eq!(response.messages.len(), 3);
        assert_eq!(
            response.messages,
            expected_transfer_msgs
        );

        // Query and verify the assets
        let assets: Vec<Addr> = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::Assets {}).unwrap()
        ).unwrap();

        assert_eq!(
            assets,
            initialize_msg.assets
                .iter()
                .map(|asset| Addr::unchecked(asset))
                .collect::<Vec<Addr>>()
        );

        // Query and verify the weights
        let weights: Vec<u64> = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::Weights {}).unwrap()
        ).unwrap();

        assert_eq!(
            weights,
            initialize_msg.weights
        );

        // Query and verify the security limit
        let max_limit_capacity: U256 = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::GetLimitCapacity {}).unwrap()
        ).unwrap();

        assert_eq!(
            max_limit_capacity,
            U256::from(weights.iter().sum::<u64>()) * LN2
        );

        // Query and verify the pool token supply
        let pool_token_supply: Uint128 = from_binary::<TokenInfoResponse>(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::TokenInfo {}).unwrap()
        ).unwrap().total_supply;

        assert_eq!(
            pool_token_supply,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify the pool tokens of the depositor
        let depositor_pool_tokens: Uint128 = from_binary::<BalanceResponse>(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::Balance { address: DEPOSITOR_ADDR.to_string() }).unwrap()
        ).unwrap().balance;

        assert_eq!(
            depositor_pool_tokens,
            Uint128::from(1000000000000000000u64)
        );

        // Query and verify escrow totals are intialized
        initialize_msg.assets
            .iter()
            .for_each(|asset| {
                let total_escrowed_balance: Uint128 = from_binary(
                    &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::TotalEscrowedAsset { asset: asset.to_string() }).unwrap()
                ).unwrap();

                assert_eq!(
                    total_escrowed_balance,
                    Uint128::zero()
                );
            });

        let total_escrowed_liquidity: Uint128 = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::TotalEscrowedLiquidity {}).unwrap()
        ).unwrap();

        assert_eq!(
            total_escrowed_liquidity,
            Uint128::zero()
        );

    }


    #[test]
    fn test_initialize_twice() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(3000u64)],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };

        // Initialize swap curves
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.clone().into()
        ).unwrap();


        // Tested action: initialize swap curves twice
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure second initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


    #[test]
    fn test_no_assets() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec![],
            assets_balances: vec![],
            weights: vec![],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves without assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::GenericError {}
        ));

    }


    #[test]
    fn test_too_many_assets() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string(), "asset_4".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(3000u64), Uint128::from(4000u64)],
            weights: vec![1u64, 1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves with too many assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::GenericError {}
        ));

    }


    #[test]
    fn test_invalid_asset_balances_count() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64)],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves with too many assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_zero_asset_balance() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(0u64)],
            weights: vec![1u64, 1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves with too many assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_invalid_weights_count() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(3000u64)],
            weights: vec![1u64, 1u64],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves with too many assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_zero_weight() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(3000u64)],
            weights: vec![1u64, 1u64, 0u64],
            amp: 1000000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves with too many assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::GenericError {}
        ));
        
    }


    #[test]
    fn test_invalid_amp() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let initialize_msg = InitializeSwapCurvesMockMsg {
            assets: vec!["asset_1".to_string(), "asset_2".to_string(), "asset_3".to_string()],
            assets_balances: vec![Uint128::from(1000u64), Uint128::from(2000u64), Uint128::from(3000u64)],
            weights: vec![1u64, 1u64, 1u64],
            amp: 900000000000000000u64,
            depositor: DEPLOYER_ADDR.to_string()
        };


        // Tested action: initialize swap curves with too many assets
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &[]),
            initialize_msg.into()
        );


        // Make sure initialization fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::InvalidAmplification {}
        ));
        
    }


}