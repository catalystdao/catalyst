mod test_amplified_deposit{
    use catalyst_types::I256;
    use cosmwasm_std::{Uint128, Addr, Attribute};
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT, event::format_vec_for_event};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::get_response_attribute, token::{deploy_test_tokens, transfer_tokens, set_token_allowance, query_token_balance, query_token_info}, definitions::{SETUP_MASTER, DEPOSITOR}, contract::{mock_factory_deploy_vault, DEFAULT_TEST_VAULT_FEE}};

    use crate::{msg::AmplifiedExecuteMsg, tests::{helpers::{compute_expected_deposit_mixed, amplified_vault_contract_storage}, parameters::{AMPLIFICATION, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_deposit_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);let 
        vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define deposit config
        let deposit_percentage = 0.15;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|vault_balance| {
                f64_to_uint128(
                    uint128_to_f64(*vault_balance) * deposit_percentage
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
            &AmplifiedExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the vault tokens return
        // NOTE: the way in which the `vault_fee` is applied when depositing results in a slightly fewer return than the 
        // one computed by `expected_return` (i.e. the fee is not applied directly to the input assets in the vault implementation)
        let observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "mint"
        ).unwrap();

        let expected_return = uint128_to_f64(INITIAL_MINT_AMOUNT) * deposit_percentage * (1. - (DEFAULT_TEST_VAULT_FEE.u64() as f64)/1e18);

        assert!(uint128_to_f64(observed_return) <= expected_return * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return * 0.98);      // Allow some margin because of the `vault_fee`


        // Verify the deposited assets have been transferred from the swapper to the vault
        vault_tokens.iter()
            .for_each(|asset| {
                let swapper_asset_balance = query_token_balance(&mut app, Addr::unchecked(asset), DEPOSITOR.to_string());
                assert_eq!(
                    swapper_asset_balance,
                    Uint128::zero()
                );

            });

        // Verify the deposited assets have been received by the vault
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
        
        // Verify the vault tokens have been minted to the depositor
        let depositor_vault_tokens_balance = query_token_balance(&mut app, vault.clone(), DEPOSITOR.to_string());
        assert_eq!(
            depositor_vault_tokens_balance,
            observed_return
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT + observed_return
        );

    }


    #[test]
    fn test_deposit_event() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);let 
        vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define deposit config
        let deposit_percentage = 0.15;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|vault_balance| {
                f64_to_uint128(
                    uint128_to_f64(*vault_balance) * deposit_percentage
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
            &AmplifiedExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Check the event
        let event = result.events[1].clone();

        assert_eq!(event.ty, "wasm-deposit");
        
        assert_eq!(
            event.attributes[1],
            Attribute::new("to_account", DEPOSITOR)
        );

        //NOTE: 'mint' is indirectly checked on `test_deposit_calculation`

        assert_eq!(
            event.attributes[3],
            Attribute::new("deposit_amounts", format_vec_for_event(deposit_amounts))
        );

    }


    #[test]
    fn test_deposit_mixed_with_zero_balance() {
        // NOTE: It is very important to test depositing an asset with a zero balance, as cw20 does not allow 
        // for asset transfers with a zero-valued balance.

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define deposit config
        let deposit_percentages = vec![0., 0.1, 0.3][..TEST_VAULT_ASSET_COUNT].to_vec();
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .zip(&deposit_percentages)
            .map(|(vault_balance, deposit_percentage)| {
                f64_to_uint128(
                    uint128_to_f64(*vault_balance) * deposit_percentage
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
            &AmplifiedExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the vault tokens return
        let observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "mint"
        ).unwrap();

        let expected_return = compute_expected_deposit_mixed(
            deposit_amounts,
            vault_weights,
            vault_initial_balances,
            INITIAL_MINT_AMOUNT,
            I256::zero(),           // Unit tracker: should be at 0, since no swaps have been executed yet
            Some(DEFAULT_TEST_VAULT_FEE),
            AMPLIFICATION
        );
        
        assert!(uint128_to_f64(observed_return) <= expected_return * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return * 0.999999);      // Allow some margin because of the `vault_fee`

    }


    #[test]
    fn test_deposit_zero_balance() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define deposit config
        let deposit_amounts: Vec<Uint128> = vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT];



        // Tested action: deposit
        let result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &AmplifiedExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the vault tokens return
        let observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "mint"
        ).unwrap();

        let expected_return = Uint128::zero();

        assert_eq!(
            observed_return,
            expected_return
        );

        // Verify no vault tokens have been received by the depositor
        assert_eq!(
            query_token_balance(&mut app, vault, DEPOSITOR.to_string()),
            Uint128::zero()
        );

    }


    #[test]
    fn test_deposit_min_out() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define deposit config
        let deposit_percentage = 0.05;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|vault_balance| {
                f64_to_uint128(
                    uint128_to_f64(*vault_balance) * deposit_percentage
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
        let expected_return = uint128_to_f64(INITIAL_MINT_AMOUNT) * deposit_percentage * (1. - (DEFAULT_TEST_VAULT_FEE.u64() as f64)/1e18);

        // Set min_out_valid to be slightly smaller than the expected return
        let min_out_valid = f64_to_uint128(expected_return * 0.99).unwrap();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = f64_to_uint128(expected_return * 1.01).unwrap();



        // Tested action 1: deposit with min_out > expected_return fails
        let response_result = app.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &AmplifiedExecuteMsg::DepositMixed {
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
            &AmplifiedExecuteMsg::DepositMixed {
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
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);let 
        vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define deposit config
        let deposit_percentage = 0.25;
        let deposit_amounts: Vec<Uint128> = vault_initial_balances.iter()
            .map(|vault_balance| {
                f64_to_uint128(
                    uint128_to_f64(*vault_balance) * deposit_percentage
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
            &AmplifiedExecuteMsg::DepositMixed {
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