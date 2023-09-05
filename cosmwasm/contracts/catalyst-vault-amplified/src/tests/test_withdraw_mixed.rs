mod test_amplified_withdraw_mixed {
    use std::str::FromStr;

    use catalyst_types::I256;
    use cosmwasm_std::{Uint128, Addr, Uint64, Attribute};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT, asset::Asset};
    use fixed_point_math::WAD;
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::get_response_attribute, token::{transfer_tokens, query_token_balance, query_token_info}, definitions::{SETUP_MASTER, WITHDRAWER}, contract::mock_factory_deploy_vault, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::TestEnv;
    use crate::{msg::AmplifiedExecuteMsg, tests::{helpers::{compute_expected_withdraw_mixed, amplified_vault_contract_storage}, parameters::{AMPLIFICATION, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_withdraw_mixed_calculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.][3-TEST_VAULT_ASSET_COUNT..].to_vec();
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );
    

    
        // Tested action: withdraw mixed
        let result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
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

        let expected_returns = compute_expected_withdraw_mixed(
            withdraw_amount,
            withdraw_ratio,
            vault_weights.clone(),
            vault_initial_balances.clone(),
            INITIAL_MINT_AMOUNT,
            I256::zero(),
            AMPLIFICATION
        );
    
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
    fn test_withdraw_mixed_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.][3-TEST_VAULT_ASSET_COUNT..].to_vec();
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );
    

    
        // Tested action: withdraw mixed
        let result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
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


    /// This test case is here to acknowledge that withdraw mixed will fail for a zero valued withdraw 
    /// amount (and a non-zero withdraw_ratio). This behavior is the same with the EVM implementation.
    /// Note that the withdrawal will go through for an all-zero withdraw ratio.
    #[test]
    fn test_withdraw_mixed_zero_balance() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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
        let withdraw_ratio_f64 = vec![1./3., 1./2., 1.][3-TEST_VAULT_ASSET_COUNT..].to_vec();
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();
    

    
        // Tested action 1: withdraw mixed with zero amount and non-zero withdraw ratio
        let response_result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        );

        // Verify the action fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::WithdrawRatioNotZero {}
        ));
    

    
        // Tested action 2: withdraw mixed with zero amount and zero withdraw ratio
        let result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![Uint64::zero(); TEST_VAULT_ASSET_COUNT],
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
    fn test_withdraw_mixed_min_out() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.][3-TEST_VAULT_ASSET_COUNT..].to_vec();
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
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
            INITIAL_MINT_AMOUNT,
            I256::zero(),
            AMPLIFICATION
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
        let response_result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
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
                if (
                    uint128_to_f64(err_out) < expected_return[0] * 1.01 &&
                    uint128_to_f64(err_out) > expected_return[0] * 0.99 &&
                    err_min_out == min_out_invalid[0]
                )
        ));
    

    
        // Tested action 2: 'withdraw mixed' with min_out <= expected_return succeeds
        env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio,
                min_out: min_out_valid
            },
            vec![],
            vec![]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_withdraw_mixed_min_out_for_0_ratio() {
        // Test specifically the 'min_out' logic for an asset with a 0-valued withdraw ratio,
        // as the 'min_out' logic for this case is implemented differently than for non-zero ratios.

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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
        let withdraw_ratio_f64 = vec![0., 0.2, 1.];        // ! The ratio for the first asset is set to 0
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // Fund withdrawer with vault tokens
        transfer_tokens(
            env.get_app(),
            withdraw_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            WITHDRAWER.to_string()
        );


    
        // Tested action: withdraw mixed fails for ratio == 0 and min_out != 0
        let response_result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
                min_out: vec![Uint128::MAX, Uint128::zero(), Uint128::zero()]   // ! Non-zero min_out specified for the first asset
            },
            vec![],
            vec![]
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
        env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            vec![],
            vec![]
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_withdraw_mixed_with_no_funds() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.][3-TEST_VAULT_ASSET_COUNT..].to_vec();
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();

        // ! Do not fund the withdrawer with vault tokens
    

    
        // Tested action: withdraw mixed
        let response_result = env.execute_contract(
            Addr::unchecked(WITHDRAWER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            format!("Error: Burn failed: Overflow: Cannot Sub with 0 and {}", withdraw_amount)
        );

    }
    

    #[test]
    fn test_withdraw_mixed_invalid_ratios() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
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


    
        // Tested action 1: invalid withdraw ratio length (too short)
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 1.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::one()]
            },
            vec![],
            vec![]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidParameters { reason: err_reason }
                if err_reason == "Invalid withdraw_ratio/min_out count.".to_string()
        ));


    
        // Tested action 2: invalid withdraw ratio length (too long)
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 0., 0., 1.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::one()]
            },
            vec![],
            vec![]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidParameters { reason: err_reason }
                if err_reason == "Invalid withdraw_ratio/min_out count.".to_string()
        ));

    
        // Tested action 3: withdraw ratio all zero
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![Uint64::zero(), Uint64::zero(), Uint64::zero()],
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            vec![],
            vec![]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnusedUnitsAfterWithdrawal { units: _ }
        ));

    
        // Tested action 4: withdraw ratio larger than 1
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 0.5, 1.2].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            vec![],
            vec![]
        );
    
        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "Arithmetic error"
        );

    
        // Tested action 5: withdraw ratio without 1
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 0.5, 0.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            vec![],
            vec![]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnusedUnitsAfterWithdrawal { units: _ }
        ));

    
        // Tested action 5: withdraw ratio with non-zero value after 1
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![0.5, 1., 0.5].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            vec![],
            vec![]
        );
    
        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::WithdrawRatioNotZero {}
        ));


        // Make sure withdrawal works with a valid withdraw_ratio
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: vec![1./3., 1./2., 1.].iter().map(|ratio| ((ratio * 1e18) as u64).into()).collect::<Vec<Uint64>>(),
                min_out: vec![Uint128::zero(), Uint128::zero(), Uint128::zero()]
            },
            vec![],
            vec![]
        ).unwrap();     // Make sure transaction succeeds

    }

}