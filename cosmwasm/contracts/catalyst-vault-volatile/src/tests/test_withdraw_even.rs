mod test_volatile_withdraw_even {
    use std::str::FromStr;

    use cosmwasm_std::{Uint128, Addr, Attribute};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::get_response_attribute, token::{query_token_balance, query_token_info, transfer_tokens}, definitions::{SETUP_MASTER, WITHDRAWER}, contract::mock_factory_deploy_vault, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::TestEnv;
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_withdraw_even_calculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );
    

    
        // Tested action: withdraw all
        let result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the returned assets
        let observed_returns = get_response_attribute::<String>(
            result.events[1].clone(),
            "withdraw_amounts"
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


        // Verify the withdrawn assets have been sent by the vault and received by the withdrawer
        vault_assets.iter()
            .zip(&vault_initial_balances)
            .zip(&observed_returns)
            .for_each(|((asset, initial_vault_balance), withdraw_amount) | {

                // Vault balance
                let vault_balance = asset.query_balance(env.get_app(), vault.to_string());
                assert_eq!(
                    vault_balance,
                    *initial_vault_balance - withdraw_amount
                );

                // Withdrawer balance
                let withdrawer_balance = asset.query_balance(env.get_app(), WITHDRAWER.to_string());
                assert_eq!(
                    withdrawer_balance,
                    *withdraw_amount
                );

            });


        // Verify the vault tokens have been burnt
        let withdrawer_vault_tokens_balance = query_token_balance(env.get_app(), vault.clone(), WITHDRAWER.to_string());
        assert_eq!(
            withdrawer_vault_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(env.get_app(), vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT - withdraw_amount
        );

    }


    #[test]
    fn test_withdraw_even_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );
    

    
        // Tested action: withdraw all
        let result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        ).unwrap();



        // Check the event
        let event = result.events[1].clone();

        assert_eq!(event.ty, "wasm-withdraw");

        assert_eq!(
            event.attributes[1],
            Attribute::new("to_account", WITHDRAWER)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("burn", withdraw_amount)
        );

        // NOTE: 'withdraw_amounts' is indirectly checked on `test_withdraw_even_calculation`

    }


    #[test]
    fn test_withdraw_even_zero_balance() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define withdraw config
        let withdraw_amount = Uint128::zero();
    

    
        // Tested action: withdraw all
        let result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the returned assets
        let observed_returns = get_response_attribute::<String>(
            result.events[1].clone(),
            "withdraw_amounts"
        ).unwrap()
            .split(", ")
            .map(Uint128::from_str)
            .collect::<Result<Vec<Uint128>, _>>()
            .unwrap();

        let expected_returns = vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT];

        assert_eq!(
            observed_returns,
            expected_returns
        );

        // Verify no assets have been received by the withdrawer
        vault_assets.iter().for_each(|token| {
            assert_eq!(
                token.query_balance(env.get_app(), WITHDRAWER.to_string()),
                Uint128::zero()
            );
        });

    }


    #[test]
    fn test_withdraw_even_min_out() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
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
        let response_result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: min_out_invalid.clone()
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        // NOTE: the min_out error will be triggered by the first asset to not fulfil the limit (i.e. the first asset in this example)
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficient { out: err_out, min_out: err_min_out }
                if err_out == f64_to_uint128(expected_return[0]).unwrap() && err_min_out == min_out_invalid[0]
        ));
    

    
        // Tested action 2: withdraw all with min_out == expected_return succeeds
        env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: min_out_valid
            },
            vec![],
            vec![]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_withdraw_even_with_no_funds() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        );

        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();

        // ! Do not fund the withdrawer with vault tokens
    

    
        // Tested action: withdraw all
        let response_result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &VolatileExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            format!("Cannot Sub with 0 and {}", withdraw_amount)
        );

    }
    

}