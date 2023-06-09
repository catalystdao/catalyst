mod test_volatile_deposit{
    use cosmwasm_std::{Uint128, Addr, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{SETUP_MASTER, deploy_test_tokens, WAD, set_token_allowance, DEFAULT_TEST_POOL_FEE, query_token_balance, transfer_tokens, DEPOSITOR, get_response_attribute, query_token_info, compute_expected_deposit_mixed, mock_factory_deploy_vault}, math_helpers::{uint128_to_f64, f64_to_uint128}}};


    //TODO add test for the deposit event


    #[test]
    fn test_deposit_calculation() {

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

        // Define deposit config
        let deposit_percentage = 0.15;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|pool_balance| {
                f64_to_uint128(
                    uint128_to_f64(*pool_balance) * deposit_percentage
                ).unwrap()
            }).collect();

        // Fund swapper with tokens and set vault allowance
        vault_tokens.iter()
            .zip(&deposit_amounts)
            .for_each(|(asset, deposit_amount)| {
                
                transfer_tokens(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );

                set_token_allowance(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(DEPOSITOR),
                    vault.to_string()
                );
            });



        // Tested action: deposit
        let result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the pool tokens return
        // NOTE: the way in which the `vault_fee` is applied when depositing results in a slightly fewer return than the 
        // one computed by `expected_return` (i.e. the fee is not applied directly to the input assets in the pool implementation)
        let observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "mint"
        ).unwrap();

        let expected_return = uint128_to_f64(INITIAL_MINT_AMOUNT) * deposit_percentage * (1. - (DEFAULT_TEST_POOL_FEE.u64() as f64)/1e18);

        assert!(uint128_to_f64(observed_return) <= expected_return * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return * 0.98);      // Allow some margin because of the `vault_fee`


        // Verify the deposited assets have been transferred from the swapper to the pool
        vault_tokens.iter()
            .for_each(|asset| {
                let swapper_asset_balance = query_token_balance(&mut app, Addr::unchecked(asset), DEPOSITOR.to_string());
                assert_eq!(
                    swapper_asset_balance,
                    Uint128::zero()
                );

            });

        // Verify the deposited assets have been received by the pool
        vault_tokens.iter()
            .zip(&vault_initial_balances)
            .zip(&deposit_amounts)
            .for_each(|((asset, vault_balance), deposit_amount)| {
                let vault_from_asset_balance = query_token_balance(&mut app, Addr::unchecked(asset), vault.to_string());
                assert_eq!(
                    vault_from_asset_balance,
                    *vault_balance + *deposit_amount
                );

            });
        
        // Verify the pool tokens have been minted to the depositor
        let depositor_pool_tokens_balance = query_token_balance(&mut app, vault.clone(), DEPOSITOR.to_string());
        assert_eq!(
            depositor_pool_tokens_balance,
            observed_return
        );
    
        // Verify the vault total pool tokens supply
        let pool_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            pool_token_info.total_supply,
            INITIAL_MINT_AMOUNT + observed_return
        );

    }


    #[test]
    fn test_deposit_mixed_with_zero_balance() {
        // NOTE: It is very important to test depositing an asset with a zero balance, as cw20 does not allow 
        // for asset transfers with a zero-valued balance.

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

        // Define deposit config
        let deposit_percentages = vec![0.1, 0., 0.3];
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .zip(&deposit_percentages)
            .map(|(pool_balance, deposit_percentage)| {
                f64_to_uint128(
                    uint128_to_f64(*pool_balance) * deposit_percentage
                ).unwrap()
            }).collect();

        // Fund swapper with tokens and set vault allowance
        vault_tokens.iter()
            .zip(&deposit_amounts)
            .filter(|(_, deposit_amount)| *deposit_amount != Uint128::zero())
            .for_each(|(asset, deposit_amount)| {
                
                transfer_tokens(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );

                set_token_allowance(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(DEPOSITOR),
                    vault.to_string()
                );
            });



        // Tested action: deposit
        let result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the pool tokens return
        let observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "mint"
        ).unwrap();

        let expected_return = compute_expected_deposit_mixed(
            deposit_amounts,
            vault_weights,
            vault_initial_balances,
            INITIAL_MINT_AMOUNT,
            Some(DEFAULT_TEST_POOL_FEE)
        );

        assert!(uint128_to_f64(observed_return) <= expected_return * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return * 0.999999);      // Allow some margin because of the `vault_fee`

    }


    //TODO this test currently fails as minting a zero-valued amount is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_deposit_zero_balance() {

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

        // Define deposit config
        let deposit_amounts: Vec<Uint128> = vec![Uint128::zero(), Uint128::zero(), Uint128::zero()];



        // Tested action: deposit
        let result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the pool tokens return
        let observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "mint"
        ).unwrap();

        let expected_return = Uint128::zero();

        assert_eq!(
            observed_return,
            expected_return
        );

    }


    #[test]
    fn test_deposit_min_out() {

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

        // Define deposit config
        let deposit_percentage = 0.05;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|pool_balance| {
                f64_to_uint128(
                    uint128_to_f64(*pool_balance) * deposit_percentage
                ).unwrap()
            }).collect();

        // Fund swapper with tokens and set vault allowance
        vault_tokens.iter()
            .zip(&deposit_amounts)
            .for_each(|(asset, deposit_amount)| {
                
                transfer_tokens(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );

                set_token_allowance(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(DEPOSITOR),
                    vault.to_string()
                );
            });

        // Compute the expected return
        let expected_return = uint128_to_f64(INITIAL_MINT_AMOUNT) * deposit_percentage * (1. - (DEFAULT_TEST_POOL_FEE.u64() as f64)/1e18);

        // Set min_out_valid to be slightly smaller than the expected return
        let min_out_valid = f64_to_uint128(expected_return * 0.99).unwrap();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = f64_to_uint128(expected_return * 1.01).unwrap();



        // Tested action 1: deposit with min_out > expected_return fails
        let response_result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: min_out_invalid
            },
            &[]
        );
        


        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficient { min_out: err_min_out, out: err_out}
                if err_min_out == min_out_invalid && err_out < err_min_out
        ));



        // Tested action 2: deposit with min_out <= expected_return succeeds
        app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: min_out_valid
            },
            &[]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_deposit_no_allowance() {

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

        // Define deposit config
        let deposit_percentage = 0.25;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|pool_balance| {
                f64_to_uint128(
                    uint128_to_f64(*pool_balance) * deposit_percentage
                ).unwrap()
            }).collect();

        // Fund swapper with tokens and set vault allowance
        vault_tokens.iter()
            .zip(&deposit_amounts)
            .for_each(|(asset, deposit_amount)| {
                
                transfer_tokens(
                    &mut app,
                    *deposit_amount,
                    Addr::unchecked(asset),
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );

                // ! Do not set token allowance
            });



        // Tested action: deposit
        let response_result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        );



        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "No allowance for this account".to_string()
        );

    }

}