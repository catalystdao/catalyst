mod test_payments_executor {

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_dependencies_with_balance};
    use cosmwasm_std::{Uint128, to_binary, Addr, Coin};

    use test_helpers::definitions::SETUP_MASTER;
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;

    use crate::commands::CommandResult;
    use crate::error::ContractError;
    use crate::executors::payments::payments_executors::{SweepCommand, execute_sweep};
    use crate::executors::types::types::Account;
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
            &to_binary(&SweepCommand {
                denoms,
                recipient: Account::Address(RECIPIENT.to_string()),
                minimum_amounts: vec![Uint128::zero(), Uint128::zero()]
            }).unwrap()
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
            &to_binary(&SweepCommand {
                denoms,
                recipient: Account::Address(RECIPIENT.to_string()),
                minimum_amounts: vec![Uint128::zero(), Uint128::zero()]
            }).unwrap()
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

        // NOTE: Important to test the following behavior, as the bank module does not
        // allow messages with empty coin vectors.



        // Tested action
        // NOTE: Using `mock_dependencies` and `mock_env` effectively results in the 'execution' 
        // of the following command to **NOT** be within the `cw_multi_test::App` simulation
        // logic. This is fine as long as any required application state is replicated on the 
        // `mock_dependencies` and `mock_env`.
        let command_result = execute_sweep(
            &mock_dependencies().as_ref(),
            &mock_env(),
            &to_binary(&SweepCommand {
                denoms: vec![],         // Empty
                recipient: Account::Address(RECIPIENT.to_string()),
                minimum_amounts: vec![] // Empty
            }).unwrap()
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
            &to_binary(&SweepCommand {
                denoms,
                recipient: Account::Address(RECIPIENT.to_string()),
                minimum_amounts: minimum_amounts.clone()
            }).unwrap()
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
            &to_binary(&SweepCommand {
                denoms,
                recipient: Account::Address(RECIPIENT.to_string()),
                minimum_amounts: vec![
                    Uint128::zero()     // Specify minimum_amounts.len != denoms.len
                ]
            }).unwrap()
        );



        // Verify the message fails the min out check
        assert!(matches!(
            command_result.err().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == "denoms/mininimum_amounts count mismatch".to_string()
        ))

    }
}