mod test_volatile_withdraw_even {
    use std::str::FromStr;

    use cosmwasm_std::{Uint128, Addr};
    use cw_multi_test::{App, Executor};
    use swap_pool_common::{ContractError, state::INITIAL_MINT_AMOUNT};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{SETUP_MASTER, deploy_test_tokens, WAD, query_token_balance, transfer_tokens, get_response_attribute, query_token_info, WITHDRAWER, mock_factory_deploy_vault}, math_helpers::{uint128_to_f64, f64_to_uint128}}};


    //TODO add test for the withdraw event

    #[test]
    fn test_withdraw_even_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![1u64, 1u64, 1u64];
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            None,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of pool tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // Fund withdrawer with pool tokens
        transfer_tokens(
            &mut app,
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );
    

    
        // Tested action: withdraw all
        let result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                pool_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        ).unwrap();



        // Verify the returned assets
        let observed_returns = get_response_attribute::<String>(
            result.events[1].clone(),
            "assets"
        ).unwrap()
            .split(", ")
            .map(Uint128::from_str)
            .collect::<Result<Vec<Uint128>, _>>()
            .unwrap();

        let expected_returns = vault_initial_balances.iter()
            .map(|balance| uint128_to_f64(*balance) * withdraw_percentage)
            .collect::<Vec<f64>>();
    
        observed_returns.iter().zip(&expected_returns)
            .for_each(|(observed_return, expected_return)| {
                assert!(uint128_to_f64(*observed_return) <= expected_return * 1.000001);
                assert!(uint128_to_f64(*observed_return) >= expected_return * 0.999999);
            });


        // Verify the withdrawn assets have been sent by the pool and received by the withdrawer
        vault_tokens.iter()
            .zip(&vault_initial_balances)
            .zip(&observed_returns)
            .for_each(|((asset, initial_vault_balance), withdraw_amount) | {

                // Vault balance
                let vault_balance = query_token_balance(&mut app, Addr::unchecked(asset), vault.to_string());
                assert_eq!(
                    vault_balance,
                    *initial_vault_balance - withdraw_amount
                );

                // Withdrawer balance
                let withdrawer_balance = query_token_balance(&mut app, Addr::unchecked(asset), WITHDRAWER.to_string());
                assert_eq!(
                    withdrawer_balance,
                    *withdraw_amount
                );

            });


        // Verify the pool tokens have been burnt
        let withdrawer_pool_tokens_balance = query_token_balance(&mut app, vault.clone(), WITHDRAWER.to_string());
        assert_eq!(
            withdrawer_pool_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total pool tokens supply
        let pool_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            pool_token_info.total_supply,
            INITIAL_MINT_AMOUNT - withdraw_amount
        );

    }


    //TODO this test currently fails as burning a zero-valued amount is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_withdraw_even_zero_balance() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![1u64, 1u64, 1u64];
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            None,
            None,
            None
        );

        // Define withdraw config
        let withdraw_amount = Uint128::zero();
    

    
        // Tested action: withdraw all
        let result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                pool_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        ).unwrap();



        // Verify the returned assets
        let observed_returns = get_response_attribute::<String>(
            result.events[1].clone(),
            "assets"
        ).unwrap()
            .split(", ")
            .map(Uint128::from_str)
            .collect::<Result<Vec<Uint128>, _>>()
            .unwrap();

        let expected_returns = vec![Uint128::zero(), Uint128::zero(), Uint128::zero()];

        assert_eq!(
            observed_returns,
            expected_returns
        );

    }


    #[test]
    fn test_withdraw_even_min_out() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![1u64, 1u64, 1u64];
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            None,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of pool tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // Fund withdrawer with pool tokens
        transfer_tokens(
            &mut app,
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );

        // Compute the expected return
        let expected_return = vault_initial_balances.iter()
            .map(|balance| uint128_to_f64(*balance) * withdraw_percentage)
            .collect::<Vec<f64>>();

        // Set min_out_valid to the expected return
        let min_out_valid = expected_return.iter()
            .map(|amount| f64_to_uint128(*amount).unwrap())
            .collect::<Vec<Uint128>>();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = expected_return.iter()
            .map(|amount| f64_to_uint128(amount * 1.01).unwrap())
            .collect::<Vec<Uint128>>();


    
        // Tested action 1: 'withdraw all' with min_out > expected_return fails
        let response_result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                pool_tokens: withdraw_amount,
                min_out: min_out_invalid.clone()
            },
            &[]
        );



        // Make sure the transaction fails
        // NOTE: the min_out error will be triggered by the first asset to not fulfil the limit (i.e. the first asset in this example)
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficient { out: err_out, min_out: err_min_out }
                if err_out == f64_to_uint128(expected_return[0]).unwrap() && err_min_out == min_out_invalid[0]
        ));
    

    
        // Tested action 2: withdraw all with min_out == expected_return succeeds
        app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                pool_tokens: withdraw_amount,
                min_out: min_out_valid
            },
            &[]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_withdraw_even_with_no_funds() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![1u64, 1u64, 1u64];
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            None,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of pool tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // ! Do not fund the withdrawer with pool tokens
    

    
        // Tested action: withdraw all
        let response_result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                pool_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        );



        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            format!("Cannot Sub with 0 and {}", withdraw_amount)
        );

    }
    

}