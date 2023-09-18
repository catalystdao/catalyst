mod test_payments_executor {

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_dependencies_with_balance, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{Uint128, Addr, Coin};

    use test_helpers::definitions::SETUP_MASTER;
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;

    use crate::commands::CommandResult;
    use crate::error::ContractError;
    use crate::executors::payments::{execute_sweep, execute_transfer, execute_pay_portion, execute_balance_check, execute_sweep_all};
    use crate::executors::types::{Account, CoinAmount};
    use crate::tests::helpers::{ROUTER, RECIPIENT, run_command_result, fund_account};



    // Sweep Tests
    // ********************************************************************************************

    #[test]
    fn test_sweep_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        // Include a zero amount to verify the executor filters out any zero-valued
        // queried balance
        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms,
            vec![Uint128::zero(), Uint128::zero()],
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify the result message works
        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            router_funds.clone()
        );
        run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify the funds have been received by the recipient
        router_funds.into_iter()
            .for_each(|coin| {
                assert_eq!(
                    test_env.get_app()
                        .wrap()
                        .query_balance(RECIPIENT, coin.denom.clone())
                        .unwrap(),
                    coin
                )
            })

    }


    #[test]
    fn test_sweep_command_all_zero() {

        // NOTE: Important to test the following behavior, as the bank module does not
        // allow messages with empty coin vectors.

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep(
            &mock_dependencies().as_ref(),  // Do not 'fund' the router
            &mock_env(),
            denoms,
            vec![Uint128::zero(), Uint128::zero()],
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify no 'CosmosMsg' is generated.
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_sweep_command_empty() {

        // NOTE: This test acknowledges that the command with empty inputs will be successful.



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep(
            &mock_dependencies().as_ref(),
            &mock_env(),
            vec![],         // Empty
            vec![],         // Empty
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify no 'CosmosMsg' is generated.
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_sweep_command_invalid_min_out() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        // Include a zero amount to verify the executor filters out any zero-valued
        // queried balance
        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let minimum_amounts = vec![
            router_funds[0].amount + Uint128::one(),    // Specify too large min out
            Uint128::zero()
        ];
        let command_result = execute_sweep(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms,
            minimum_amounts.clone(),
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify the message fails the min out check
        assert!(matches!(
            command_result,
            CommandResult::Check(check_result)
                if check_result.clone().err().unwrap() == format!(
                    "Minimum amount {} not fulfilled on sweep operation (found {}{})",
                    router_funds[0],
                    minimum_amounts[0],
                    router_funds[0].denom
                )
        ))

    }


    #[test]
    fn test_sweep_command_invalid_params() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        // Include a zero amount to verify the executor filters out any zero-valued
        // queried balance
        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms,
            vec![Uint128::zero()],     // Specify minimum_amounts.len != denoms.len
            Account::Address(RECIPIENT.to_string())
        );



        // Verify the command fails
        assert!(matches!(
            command_result.err().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == "denoms/minimum_amounts count mismatch".to_string()
        ))

    }



    // Sweep All Tests
    // ********************************************************************************************

    #[test]
    fn test_sweep_all_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();

        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(5678u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep_all(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify the result message works
        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            router_funds.clone()
        );
        run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify the funds have been received by the recipient
        router_funds.into_iter()
            .for_each(|coin| {
                assert_eq!(
                    test_env.get_app()
                        .wrap()
                        .query_balance(RECIPIENT, coin.denom.clone())
                        .unwrap(),
                    coin
                )
            });

        // Verify the router has no funds left
        let router_coins = test_env.get_app()
            .wrap()
            .query_all_balances(ROUTER.to_string())
            .unwrap();
        assert!(router_coins.len() == 0);

    }


    #[test]
    fn test_sweep_all_command_no_funds() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep_all(
            &mock_dependencies().as_ref(),      // ! Do not set any funds for the router
            &mock_env(),
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify the result message works
        run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify the router has no funds
        let router_coins = test_env.get_app()
            .wrap()
            .query_all_balances(ROUTER.to_string())
            .unwrap();
        assert!(router_coins.len() == 0);

    }



    // Transfer Tests
    // ********************************************************************************************

    #[test]
    fn test_transfer_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();

        // Include a zero amount to verify the executor filters out any zero-valued
        // balance.
        let transfer_coins = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_transfer(
            &mock_dependencies_with_balance(&transfer_coins).as_ref(),
            &mock_env(),
            transfer_coins.iter()
                .map(|coin| CoinAmount::Coin(coin.clone()))
                .collect(),
            Account::Address(RECIPIENT.to_string()),
        ).unwrap();



        // Verify the result message works
        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            transfer_coins.clone()
        );
        run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify the funds have been received by the recipient
        transfer_coins.into_iter()
            .for_each(|coin| {
                assert_eq!(
                    test_env.get_app()
                        .wrap()
                        .query_balance(RECIPIENT, coin.denom.clone())
                        .unwrap(),
                    coin
                )
            })

    }


    #[test]
    fn test_transfer_command_all_zero() {

        // NOTE: Important to test the following behavior, as the bank module does not
        // allow messages with empty coin vectors.

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();

        // All coins set to 0
        let transfer_amounts = vec![
            Coin::new(0u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];


        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_transfer(
            &mock_dependencies().as_ref(),
            &mock_env(),
            transfer_amounts.iter()
                .map(|coin| CoinAmount::Coin(coin.clone()))
                .collect(),
            Account::Address(RECIPIENT.to_string()),
        ).unwrap();



        // Verify no 'CosmosMsg' is generated.
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_transfer_command_empty() {

        // NOTE: This test acknowledges that the command with empty inputs will be successful.



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_transfer(
            &mock_dependencies().as_ref(),
            &mock_env(),
            vec![],    // Empty
            Account::Address(RECIPIENT.to_string()),
        ).unwrap();



        // Verify no 'CosmosMsg' is generated.
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }



    // Pay Portion Tests
    // ********************************************************************************************

    #[test]
    fn test_pay_portion_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        // Include a zero amount to verify the executor filters out any zero-valued
        // queried balance
        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];
        let pay_portion = vec![0.6, 0.7];
        let pay_portion_bips: Vec<Uint128> = pay_portion.iter()
            .map(|portion| Uint128::new((portion * 10000.) as u128))
            .collect();



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_pay_portion(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms,
            pay_portion_bips,
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify the result message works
        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            router_funds.clone()
        );
        run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify the funds have been received by the recipient
        router_funds.into_iter()
            .zip(pay_portion)
            .for_each(|(coin, portion)| {

                let expected_coin = Coin::new(
                    (coin.amount.u128() as f64 * portion) as u128,
                    coin.denom
                );

                assert_eq!(
                    test_env.get_app()
                        .wrap()
                        .query_balance(RECIPIENT, expected_coin.denom.clone())
                        .unwrap(),
                    expected_coin
                )
            });

    }


    #[test]
    fn test_pay_portion_command_all_zero() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        // All funds set to 0
        let router_funds = vec![
            Coin::new(0u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];
        let pay_portion = vec![0.6, 0.7];
        let pay_portion_bips: Vec<Uint128> = pay_portion.iter()
            .map(|portion| Uint128::new((portion * 10000.) as u128))
            .collect();



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_pay_portion(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms,
            pay_portion_bips,
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify no 'CosmosMsg' is generated.
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_pay_portion_command_empty() {

        // NOTE: This test acknowledges that the command with empty inputs will be successful.



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_pay_portion(
            &mock_dependencies().as_ref(),
            &mock_env(),
            vec![],     // Empty
            vec![],       // Empty
            Account::Address(RECIPIENT.to_string())
        ).unwrap();



        // Verify no 'CosmosMsg' is generated.
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_pay_portion_command_invalid_params() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        // Include a zero amount to verify the executor filters out any zero-valued
        // queried balance
        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(0u128, assets[1].denom.clone())
        ];



        // Tested action 1: bips.len != denoms.len
        let command_result = execute_pay_portion(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms.clone(),
            vec![Uint128::new(10000u128)],
            Account::Address(RECIPIENT.to_string())
        );

        // Verify the excution fails
        assert!(matches!(
            command_result.err().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == "denoms/bips count mismatch".to_string()
        ));



        // Tested action 2: bips == 0
        let command_result = execute_pay_portion(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms.clone(),
            vec![Uint128::zero(), Uint128::new(10000u128)],   // Zero bips
            Account::Address(RECIPIENT.to_string())
        );

        // Verify the excution fails
        assert!(matches!(
            command_result.err().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == "Invalid bips.".to_string()
        ));



        // Tested action 3: bips > 1 (10000)
        let command_result = execute_pay_portion(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
            denoms.clone(),
            vec![Uint128::new(10001u128), Uint128::new(10000u128)],   // Bips > 1
            Account::Address(RECIPIENT.to_string())
        );

        // Verify the excution fails
        assert!(matches!(
            command_result.err().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == "Invalid bips.".to_string()
        ));

    }



    // Balance Check Tests
    // ********************************************************************************************

    #[test]
    fn test_balance_check_command() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(5678u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_balance_check(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
                denoms,
                vec![Uint128::zero(), Uint128::zero()],
                Account::Address(MOCK_CONTRACT_ADDR.to_string()),   // Check the balance of the router.
                                                                    // Using 'mock_env()', the router takes the address 'MOCK_CONTRACT_ADDR'
        ).unwrap();



        // Verify the check is successful
        assert!(matches!(
            command_result,
            CommandResult::Check(result)
                if result.is_ok()
        ));

    }


    #[test]
    fn test_balance_check_command_invalid_min_out() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(5678u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let minimum_amounts = vec![
            router_funds[0].amount + Uint128::one(),    // Specify too large min out
            Uint128::zero()
        ];
        let command_result = execute_balance_check(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
                denoms,
                minimum_amounts.clone(), 
                Account::Address(MOCK_CONTRACT_ADDR.to_string()),   // Check the balance of the router.
                                                                    // Using 'mock_env()', the router takes the address 'MOCK_CONTRACT_ADDR'
        ).unwrap();



        // Verify the command fails the min out check
        assert!(matches!(
            command_result,
            CommandResult::Check(check_result)
                if check_result.clone().err().unwrap() == format!(
                    "Minimum amount {}{} not fulfilled on balance check operation (found {})",
                    minimum_amounts[0],
                    router_funds[0].denom,
                    router_funds[0]
                )
        ));
    }


    #[test]
    fn test_balance_check_command_invalid_params() {

        let test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let assets =  test_env.get_assets()[..2].to_vec();
        let denoms: Vec<String> = vec![
            assets[0].denom.clone(),
            assets[1].denom.clone(),
        ];

        let router_funds = vec![
            Coin::new(1234u128, assets[0].denom.clone()),
            Coin::new(5678u128, assets[1].denom.clone())
        ];



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_balance_check(
            &mock_dependencies_with_balance(&router_funds).as_ref(),
            &mock_env(),
                denoms, 
                vec![
                    Uint128::zero()     // Specify minimum_amounts.len != denoms.len
                ].clone(), 
                Account::Address(MOCK_CONTRACT_ADDR.to_string()),   // Check the balance of the router.
                                                                    // Using 'mock_env()', the router takes the address 'MOCK_CONTRACT_ADDR'
        );



        // Verify the command fails
        assert!(matches!(
            command_result.err().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == "denoms/minimum_amounts count mismatch".to_string()
        ));
    }

}