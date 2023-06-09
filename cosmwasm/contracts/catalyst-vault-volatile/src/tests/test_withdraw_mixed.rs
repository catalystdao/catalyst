mod test_volatile_withdraw_mixed {
    use std::str::FromStr;

    use cosmwasm_std::{Uint128, Addr, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{ SETUP_MASTER, deploy_test_tokens, WAD, query_token_balance, transfer_tokens, get_response_attribute, query_token_info, WITHDRAWER, compute_expected_withdraw_mixed, mock_factory_deploy_vault}, math_helpers::{uint128_to_f64, f64_to_uint128}}};


    //TODO add test for the withdraw event

    #[test]
    fn test_withdraw_mixed_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.];
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            &mut app,
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );
    

    
        // Tested action: withdraw mixed
        let result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
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

        let expected_returns = compute_expected_withdraw_mixed(
            withdraw_amount,
            withdraw_ratio,
            vault_weights.clone(),
            vault_initial_balances.clone(),
            INITIAL_MINT_AMOUNT
        );
    
        observed_returns.iter().zip(&expected_returns)
            .for_each(|(observed_return, expected_return)| {
                assert!(uint128_to_f64(*observed_return) <= expected_return * 1.000001);
                assert!(uint128_to_f64(*observed_return) >= expected_return * 0.999999);
            });


        // Verify the withdrawn assets have been sent by the vault and received by the withdrawer
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


        // Verify the vault tokens have been burnt
        let withdrawer_vault_tokens_balance = query_token_balance(&mut app, vault.clone(), WITHDRAWER.to_string());
        assert_eq!(
            withdrawer_vault_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT - withdraw_amount
        );

    }


    //TODO this test currently fails as burning a zero-valued amount is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_withdraw_mixed_zero_balance() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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
        let withdraw_ratio_f64 = vec![1./3., 1./2., 1.];
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();
    

    
        // Tested action: withdraw mixed
        let result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio,
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
    fn test_withdraw_mixed_min_out() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.];
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            &mut app,
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );

        // Compute the expected return
        let expected_return = compute_expected_withdraw_mixed(
            withdraw_amount,
            withdraw_ratio.clone(),
            vault_weights.clone(),
            vault_initial_balances.clone(),
            INITIAL_MINT_AMOUNT
        );

        // Set min_out_valid to be slightly smaller than the expected return
        let min_out_valid = expected_return.iter()
            .map(|amount| f64_to_uint128(amount * 0.99).unwrap())
            .collect::<Vec<Uint128>>();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = expected_return.iter()
            .map(|amount| f64_to_uint128(amount * 1.01).unwrap())
            .collect::<Vec<Uint128>>();


    
        // Tested action 1: 'withdraw mixed' with min_out > expected_return fails
        let response_result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
                min_out: min_out_invalid.clone()
            },
            &[]
        );



        // Make sure the transaction fails
        // NOTE: the min_out error will be triggered by the first asset to not fulfil the limit (i.e. the first asset in this example)
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficient { out: err_out, min_out: err_min_out }
                if (
                    uint128_to_f64(err_out) < expected_return[0] * 1.01 &&
                    uint128_to_f64(err_out) > expected_return[0] * 0.99 &&
                    err_min_out == min_out_invalid[0]
                )
        ));
    

    
        // Tested action 2: 'withdraw mixed' with min_out <= expected_return succeeds
        app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio,
                min_out: min_out_valid
            },
            &[]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_withdraw_mixed_min_out_for_0_ratio() {
        // Test specifically the 'min_out' logic for an asset with a 0-valued withdraw ratio,
        // as the 'min_out' logic for this case is implemented differently than for non-zero ratios.

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();
        let withdraw_ratio_f64 = vec![0., 0.2, 1.];        // ! The ratio for the first asset is set to 0
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            &mut app,
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );


    
        // Tested action: withdraw mixed fails for ratio == 0 and min_out != 0
        let response_result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
                min_out: vec![Uint128::MAX, Uint128::zero(), Uint128::zero()]   // ! Non-zero min_out specified for the first asset
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficient { out: err_out, min_out: err_min_out }
                if (
                    err_out == Uint128::zero() &&
                    err_min_out == Uint128::MAX
                )
        ));
    
        // Make sure the withdraw ratio does work when 'min_out' is not provided
        app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_withdraw_mixed_with_no_funds() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.];
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // ! Do not fund the withdrawer with vault tokens
    

    
        // Tested action: withdraw mixed
        let response_result = app.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio,
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
    

    #[test]
    fn test_withdraw_mixed_invalid_ratios() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();


    
        // Tested action 1: invalid withdraw ratio length (too short)
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 1.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::one()]
            },
            &[]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));


    
        // Tested action 2: invalid withdraw ratio length (too long)
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 0., 0., 1.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::one()]
            },
            &[]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::GenericError {}
        ));

    
        // Tested action 3: withdraw ratio all zero
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![Uint64::zero(), Uint64::zero(), Uint64::zero()],
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnusedUnitsAfterWithdrawal { units: _ }
        ));

    
        // Tested action 4: withdraw ratio larger than 1
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 0.5, 1.2].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        );
    
        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "Arithmetic error"
        );

    
        // Tested action 5: withdraw ratio without 1
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 0.5, 0.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnusedUnitsAfterWithdrawal { units: _ }
        ));

    
        // Tested action 5: withdraw ratio with non-zero value after 1
        let response_result = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 1., 0.5].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::WithdrawRatioNotZero { ratio: err_ratio }
                if err_ratio == Uint64::new((0.5 * 1e18) as u64)
        ));


        // Make sure withdrawal works with a valid withdraw_ratio
        app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![1./3., 1./2., 1.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            &[]
        ).unwrap();     // Make sure transaction succeeds

    }

}