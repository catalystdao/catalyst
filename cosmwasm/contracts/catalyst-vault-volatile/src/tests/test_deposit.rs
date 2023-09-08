mod test_volatile_deposit{
    use cosmwasm_std::{Uint128, Addr, Attribute};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT, event::format_vec_for_event, bindings::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::get_response_attribute, definitions::{SETUP_MASTER, DEPOSITOR, VAULT_TOKEN_DENOM}, contract::{mock_factory_deploy_vault, DEFAULT_TEST_VAULT_FEE}, env::CustomTestEnv, asset::CustomTestAsset, vault_token::CustomTestVaultToken};

    use crate::tests::{TestEnv, TestVaultToken};
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{compute_expected_deposit_mixed, volatile_vault_contract_storage}, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_deposit_calculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());let 
        vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
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

        // Fund swapper with tokens
        vault_assets.iter()
            .zip(&deposit_amounts)
            .for_each(|(asset, deposit_amount)| {
                asset.transfer(
                    env.get_app(),
                    *deposit_amount,
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );
            });



        // Tested action: deposit
        let result = env.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets.clone(),
            deposit_amounts.clone()
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
        vault_assets.iter()
            .for_each(|asset| {
                let swapper_asset_balance = asset.query_balance(env.get_app(), DEPOSITOR.to_string());
                assert_eq!(
                    swapper_asset_balance,
                    Uint128::zero()
                );

            });

        // Verify the deposited assets have been received by the vault
        vault_assets.iter()
            .zip(&vault_initial_balances)
            .zip(&deposit_amounts)
            .for_each(|((asset, vault_balance), deposit_amount)| {
                let vault_from_asset_balance = asset.query_balance(env.get_app(), vault.to_string());
                assert_eq!(
                    vault_from_asset_balance,
                    *vault_balance + *deposit_amount
                );

            });
        
        // Verify the vault tokens have been minted to the depositor
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let depositor_vault_tokens_balance = vault_token.query_balance(env.get_app(), DEPOSITOR.to_string());
        assert_eq!(
            depositor_vault_tokens_balance,
            observed_return
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_supply = vault_token.total_supply(env.get_app());
        assert_eq!(
            vault_token_supply,
            INITIAL_MINT_AMOUNT + observed_return
        );

    }


    #[test]
    fn test_deposit_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());let 
        vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
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

        // Fund swapper with tokens
        vault_assets.iter()
            .zip(&deposit_amounts)
            .for_each(|(asset, deposit_amount)| {
                asset.transfer(
                    env.get_app(),
                    *deposit_amount,
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );
            });



        // Tested action: deposit
        let result = env.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets.clone(),
            deposit_amounts.clone()
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

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
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

        // Fund swapper with tokens
        vault_assets.iter()
            .zip(&deposit_amounts)
            .filter(|(_, deposit_amount)| *deposit_amount != Uint128::zero())
            .for_each(|(asset, deposit_amount)| {
                asset.transfer(
                    env.get_app(),
                    *deposit_amount,
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );
            });



        // Tested action: deposit
        let result = env.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets.clone(),
            deposit_amounts.clone()
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
            Some(DEFAULT_TEST_VAULT_FEE)
        );

        assert!(uint128_to_f64(observed_return) <= expected_return * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return * 0.999999);      // Allow some margin because of the `vault_fee`

    }


    #[test]
    fn test_deposit_zero_balance() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None,
            None
        );

        // Define deposit config
        let deposit_amounts: Vec<Uint128> = vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT];



        // Tested action: deposit
        let result = env.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets.clone(),
            deposit_amounts.clone()
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
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        assert_eq!(
            vault_token.query_balance(env.get_app(), DEPOSITOR.to_string()),
            Uint128::zero()
        );

    }


    #[test]
    fn test_deposit_min_out() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
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

        // Fund swapper with tokens
        vault_assets.iter()
            .zip(&deposit_amounts)
            .for_each(|(asset, deposit_amount)| {
                asset.transfer(
                    env.get_app(),
                    *deposit_amount,
                    Addr::unchecked(SETUP_MASTER),
                    DEPOSITOR.to_string(),
                );
            });

        // Compute the expected return
        let expected_return = uint128_to_f64(INITIAL_MINT_AMOUNT) * deposit_percentage * (1. - (DEFAULT_TEST_VAULT_FEE.u64() as f64)/1e18);

        // Set min_out_valid to be slightly smaller than the expected return
        let min_out_valid = f64_to_uint128(expected_return * 0.99).unwrap();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = f64_to_uint128(expected_return * 1.01).unwrap();



        // Tested action 1: deposit with min_out > expected_return fails
        let response_result = env.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: min_out_invalid
            },
            vault_assets.clone(),
            deposit_amounts.clone()
        );
        


        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficient { min_out: err_min_out, out: err_out}
                if err_min_out == min_out_invalid && err_out < err_min_out
        ));



        // Tested action 2: deposit with min_out <= expected_return succeeds
        env.execute_contract(
            Addr::unchecked(DEPOSITOR),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: min_out_valid
            },
            vault_assets.clone(),
            deposit_amounts.clone()
        ).unwrap();     // Make sure the transaction succeeds

    }


    #[test]
    fn test_deposit_invalid_funds() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
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



        // Tested action 1: no funds
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vec![],   // ! Do not send funds
            vec![]
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetNotReceived { asset }
                if asset == Into::<Asset>::into(vault_assets[0].clone()).to_string()  // Error corresponds to the first asset that is not received
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "No allowance for this account".to_string()
        );



        // Tested action 2: too few assets
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets[..vault_assets.len()-1].to_vec(),      // ! Send one asset less
            deposit_amounts[..deposit_amounts.len()-1].to_vec()
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetNotReceived { asset }
                if asset == Into::<Asset>::into(vault_assets[vault_assets.len()-1].clone()).to_string()  // Error corresponds to the first asset that is not received
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "No allowance for this account".to_string()
        );



        // Tested action 3: too many assets
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            env.get_assets()[..TEST_VAULT_ASSET_COUNT+1].to_vec(),      // ! Send one asset more
            deposit_amounts.iter().cloned().chain(vec![Uint128::from(1000u128)].into_iter()).collect()
        );

        // Make sure the transaction fails
        #[cfg(feature="asset_native")]
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetSurplusReceived {}
        );
        
        // NOTE: this does not error for cw20 assets, as it's just the *allowance* that is set.
        #[cfg(feature="asset_cw20")]
        assert!(response_result.is_ok());



        // Tested action 4: asset amount too low
        let mut too_low_deposit_amounts = deposit_amounts.clone();
        too_low_deposit_amounts[0] = too_low_deposit_amounts[0] - Uint128::one();

        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets.clone(),
            too_low_deposit_amounts.clone()
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnexpectedAssetAmountReceived { received_amount, expected_amount, asset }
                if
                    received_amount == too_low_deposit_amounts[0] &&
                    expected_amount == deposit_amounts[0] &&
                    asset == Into::<Asset>::into(vault_assets[0].clone()).to_string()
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            format!("Cannot Sub with {} and {}", too_low_deposit_amounts[0], deposit_amounts[0])
        );



        // Tested action 5: asset amount too high
        let mut too_high_deposit_amounts = deposit_amounts.clone();
        too_high_deposit_amounts[0] = too_high_deposit_amounts[0] + Uint128::one();

        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets.clone(),
            too_high_deposit_amounts.clone()
        );

        // Make sure the transaction fails
        #[cfg(feature="asset_native")]
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnexpectedAssetAmountReceived { received_amount, expected_amount, asset }
                if
                    received_amount == too_high_deposit_amounts[0] &&
                    expected_amount == deposit_amounts[0] &&
                    asset == Into::<Asset>::into(vault_assets[0].clone()).to_string()
        );
        
        // NOTE: this does not error for cw20 assets, as it's just the *allowance* that is set too high.
        #[cfg(feature="asset_cw20")]
        assert!(response_result.is_ok());



        // Make sure the deposit works for valid amounts
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            vault_assets,
            deposit_amounts
        ).unwrap();

    }

}