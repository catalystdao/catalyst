mod test_volatile_local_swap {
    use cosmwasm_std::{Uint128, Addr, Uint64, Attribute};
    use catalyst_vault_common::{ContractError, asset::Asset};
    use fixed_point_math::WAD;
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, definitions::{SETUP_MASTER, LOCAL_SWAPPER, FACTORY_OWNER}, contract::{mock_factory_deploy_vault, DEFAULT_TEST_VAULT_FEE, DEFAULT_TEST_GOV_FEE, mock_set_governance_fee_share}, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::TestEnv;
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{compute_expected_local_swap, volatile_vault_contract_storage}, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};


    #[test]
    fn test_local_swap_calculation() {

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
            None
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];

        // Swap 25% of the vault
        let swap_amount = from_balance * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );



        // Tested action: local swap
        let result = env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Verify the swap return
        let expected_swap = compute_expected_local_swap(
            swap_amount,
            from_weight,
            from_balance,
            to_weight,
            to_balance,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        let observed_return = result.events[1].attributes
            .iter().find(|attr| attr.key == "to_amount").unwrap()
            .value.parse::<Uint128>().unwrap();

        assert!(uint128_to_f64(observed_return) <= expected_swap.to_amount * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_swap.to_amount * 0.999999);


        // Verify the input assets have been transferred from the swapper to the vault
        let swapper_from_asset_balance = from_asset.query_balance(env.get_app(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

        // Verify the input assets have been received by the vault and the governance fee has been collected
        // Note: the vault fee calculation is indirectly tested via the governance fee calculation
        let vault_from_asset_balance = from_asset.query_balance(env.get_app(), vault.to_string());
        let factory_owner_from_asset_balance = from_asset.query_balance(env.get_app(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance + factory_owner_from_asset_balance,    // Some of the swappers balance will have gone to the factory owner (governance fee)
            from_balance + swap_amount
        );

        assert!(uint128_to_f64(factory_owner_from_asset_balance) <= expected_swap.governance_fee * 1.000001);
        assert!(uint128_to_f64(factory_owner_from_asset_balance) >= expected_swap.governance_fee * 0.999999);

        // Verify the output assets have been transferred to the swapper
        let vault_to_asset_balance = to_asset.query_balance(env.get_app(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            to_balance - observed_return
        );

        // Verify the output assets have been received by the swapper
        let swapper_to_asset_balance = to_asset.query_balance(env.get_app(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            observed_return
        );

    }


    #[test]
    fn test_local_swap_min_out() {

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
            None
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];

        // Swap 25% of the vault
        let swap_amount = vault_initial_balances[from_asset_idx] * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Compute the expected swap return
        let expected_swap = compute_expected_local_swap(
            swap_amount,
            from_weight,
            from_balance,
            to_weight,
            to_balance,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        // Set min out to be slightly larger than the expected output
        let min_out = f64_to_uint128(expected_swap.to_amount * 1.01).unwrap();



        // Tested action: local swap
        let response_result = env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        );



        // Make sure the swap fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::ReturnInsufficient { min_out: error_min_out, out }
                if error_min_out == min_out && out < min_out
        ));

        // Make sure the swap goes through if min_out is decreased
        env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: f64_to_uint128(expected_swap.to_amount * 0.99).unwrap()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();


    }
    

    #[test]
    fn test_local_swap_event() {

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
            None
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();

        // Swap 25% of the vault
        let swap_amount = from_balance * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );



        // Tested action: local swap
        let result = env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Check the event
        let local_swap_event = result.events[1].clone();
        
        assert_eq!(local_swap_event.ty, "wasm-local-swap");

        assert_eq!(
            local_swap_event.attributes[1],
            Attribute::new("account", LOCAL_SWAPPER)
        );
        assert_eq!(
            local_swap_event.attributes[2],
            Attribute::new("from_asset_ref", from_asset.get_asset_ref())
        );
        assert_eq!(
            local_swap_event.attributes[3],
            Attribute::new("to_asset_ref", to_asset.get_asset_ref())
        );
        assert_eq!(
            local_swap_event.attributes[4],
            Attribute::new("from_amount", swap_amount)
        );
    
        //NOTE: 'to_amount' is indirectly checked on `test_local_swap_calculation`

    }


    #[test]
    fn test_local_swap_from_asset_not_in_vault() {

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
            None
        );

        // Define local swap config
        let from_asset = env.get_assets()[TEST_VAULT_ASSET_COUNT+1].clone();

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();

        let swap_amount = Uint128::from(10000000u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );



        // Tested action: local swap
        let response_result = env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        );



        // Make sure the swap fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::AssetNotFound {}
        ));

    }
    

    #[test]
    fn test_local_swap_to_asset_not_in_vault() {

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
            None
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();

        let to_asset = env.get_assets()[TEST_VAULT_ASSET_COUNT+1].clone();

        let swap_amount = Uint128::from(10000000u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );



        // Tested action: local swap
        let response_result = env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        );



        // Make sure the swap fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::AssetNotFound {}
        ));

    }


    #[test]
    fn test_local_swap_zero_from_amount() {

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
            None
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();

        // Swap amount set to 0
        let swap_amount = Uint128::zero();



        // Tested action: local swap
        env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Verify no output assets have been transferred to the swapper
        let vault_to_asset_balance = to_asset.query_balance(env.get_app(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            vault_initial_balances[to_asset_idx]
        );

        // Verify no output assets have been received by the swapper
        let swapper_to_asset_balance = to_asset.query_balance(env.get_app(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_local_swap_zero_to_amount() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64), Uint128::from(3u64) * WAD.as_uint128()];   // ! Initialize to_asset's vault balance to a very small value, to force the output of swaps to be 0
        let vault_weights = vec![Uint128::one(), Uint128::one(), Uint128::one()];
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];

        // Swap 1% of the vault
        let swap_amount = vault_initial_balances[from_asset_idx] * Uint128::from(1u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Check the expected swap return is 0 (make sure the test is properly configured)
        let expected_swap = compute_expected_local_swap(
            swap_amount,
            from_weight,
            from_balance,
            to_weight,
            to_balance,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );
        assert_eq!(
            expected_swap.to_amount as u64,     // Cast to u64 to ignore any decimal places
            0u64
        );



        // Tested action: local swap
        env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Verify no output assets have been transferred to the swapper
        let vault_to_asset_balance = to_asset.query_balance(env.get_app(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            vault_initial_balances[to_asset_idx]
        );

        // Verify no output assets have been received by the swapper
        let swapper_to_asset_balance = to_asset.query_balance(env.get_app(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_local_swap_zero_gov_fee() {
        // This test verifies that zero-valued governance fees do not cause local swaps to fail.
        // This is important, as cw20 does not allow zero-valued token transfers. Hence if a governace fee
        // transfer message is set for a zero-valued governance fee, the transaction will fail

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
            None
        );

        // Set the governance fee to 0 (note the default mock vault has a non-zero governance fee)
        mock_set_governance_fee_share(
            env.get_app(),
            vault.clone(),
            Uint64::zero()
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();

        // Swap 25% of the vaultt
        let swap_amount = vault_initial_balances[from_asset_idx] * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );



        // Tested action: local swap
        env.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();


        // Verify no governance fee was collected
        let factory_owner_from_asset_balance = from_asset.query_balance(env.get_app(), FACTORY_OWNER.to_string());
        assert_eq!(
            factory_owner_from_asset_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_local_swap_invalid_funds() {

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
            None
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_assets[to_asset_idx].clone();

        // Swap 25% of the vault
        let swap_amount = from_balance * Uint128::from(25u64)/ Uint128::from(100u64);



        // Tested action 1: no funds
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
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
                if asset == Into::<Asset>::into(from_asset.clone()).to_string()
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "No allowance for this account".to_string()
        );



        // Tested action 2: invalid asset
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![to_asset.clone()],   // ! Send 'to_asset' instead of 'from_asset'
            vec![swap_amount]
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetNotReceived { asset }
                if asset == Into::<Asset>::into(from_asset.clone()).to_string()
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
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone(), to_asset.clone()],   // ! Send both 'from_asset' and 'to_asset'
            vec![swap_amount, Uint128::one()]
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
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount - Uint128::one()]
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnexpectedAssetAmountReceived { received_amount, expected_amount, asset }
                if
                    received_amount == swap_amount - Uint128::one() &&
                    expected_amount == swap_amount &&
                    asset == Into::<Asset>::into(from_asset.clone()).to_string()
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            format!("Cannot Sub with {} and {}", swap_amount - Uint128::one(), swap_amount)
        );



        // Tested action 5: asset amount too high
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount + Uint128::one()]
        );

        // Make sure the transaction fails
        #[cfg(feature="asset_native")]
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnexpectedAssetAmountReceived { received_amount, expected_amount, asset }
                if
                    received_amount == swap_amount + Uint128::one() &&
                    expected_amount == swap_amount &&
                    asset == Into::<Asset>::into(from_asset.clone()).to_string()
        );
        
        // NOTE: this does not error for cw20 assets, as it's just the *allowance* that is set too high.
        #[cfg(feature="asset_cw20")]
        assert!(response_result.is_ok());



        // Make sure the swap works for a valid amount
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();

    }

}