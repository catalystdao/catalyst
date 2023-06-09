mod test_volatile_local_swap {
    use cosmwasm_std::{Uint128, Addr, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::ContractError;

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{SETUP_MASTER, deploy_test_tokens, WAD, set_token_allowance, compute_expected_local_swap, DEFAULT_TEST_POOL_FEE, DEFAULT_TEST_GOV_FEE, query_token_balance, transfer_tokens, LOCAL_SWAPPER, FACTORY_OWNER, mock_test_token_definitions, mock_set_governance_fee_share, mock_factory_deploy_vault}, math_helpers::{uint128_to_f64, f64_to_uint128}}};


    //TODO add test for the local swap event


    #[test]
    fn test_local_swap_calculation() {

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

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];

        // Swap 25% of the pool
        let swap_amount = from_balance * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );



        // Tested action: local swap
        let result = app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let expected_swap = compute_expected_local_swap(
            swap_amount,
            from_weight,
            from_balance,
            to_weight,
            to_balance,
            Some(DEFAULT_TEST_POOL_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        let observed_return = result.events[1].attributes
            .iter().find(|attr| attr.key == "to_amount").unwrap()
            .value.parse::<Uint128>().unwrap();

        assert!(uint128_to_f64(observed_return) <= expected_swap.to_amount * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_swap.to_amount * 0.999999);


        // Verify the input assets have been transferred from the swapper to the pool
        let swapper_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

        // Verify the input assets have been received by the pool and the governance fee has been collected
        // Note: the pool fee calculation is indirectly tested via the governance fee calculation
        let vault_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), vault.to_string());
        let factory_owner_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance + factory_owner_from_asset_balance,    // Some of the swappers balance will have gone to the factory owner (governance fee)
            from_balance + swap_amount
        );

        assert!(uint128_to_f64(factory_owner_from_asset_balance) <= expected_swap.governance_fee * 1.000001);
        assert!(uint128_to_f64(factory_owner_from_asset_balance) >= expected_swap.governance_fee * 0.999999);

        // Verify the output assets have been transferred to the swapper
        let vault_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            to_balance - observed_return
        );

        // Verify the output assets have been received by the swapper
        let swapper_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            observed_return
        );

    }


    #[test]
    fn test_local_swap_min_out() {

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

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];

        // Swap 25% of the pool
        let swap_amount = vault_initial_balances[from_asset_idx] * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );

        // Compute the expected swap return
        let expected_swap = compute_expected_local_swap(
            swap_amount,
            from_weight,
            from_balance,
            to_weight,
            to_balance,
            Some(DEFAULT_TEST_POOL_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        // Set min out to be slightly larger than the expected output
        let min_out = f64_to_uint128(expected_swap.to_amount * 1.01).unwrap();



        // Tested action: local swap
        let response_result = app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out
            },
            &[]
        );



        // Make sure the swap fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::ReturnInsufficient { min_out: error_min_out, out }
                if error_min_out == min_out && out < min_out
        ));

        // Make sure the swap goes through if min_out is decreased
        app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: f64_to_uint128(expected_swap.to_amount * 0.99).unwrap()
            },
            &[]
        ).unwrap();



    }
    

    #[test]
    fn test_local_swap_from_asset_not_in_pool() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let tokens = deploy_test_tokens(&mut app, None, Some(mock_test_token_definitions(4)));
        let vault_tokens = tokens[0..3].to_vec();
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

        // Define local swap config
        let from_asset = tokens[3].clone();

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();

        let swap_amount = Uint128::from(10000000u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );



        // Tested action: local swap
        let response_result = app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            &[]
        );



        // Make sure the swap fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::InvalidAssets {}
        ));

    }
    

    #[test]
    fn test_local_swap_to_asset_not_in_pool() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let tokens = deploy_test_tokens(&mut app, None, Some(mock_test_token_definitions(4)));
        let vault_tokens = tokens[0..3].to_vec();
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

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();

        let to_asset = tokens[3].clone();

        let swap_amount = Uint128::from(10000000u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );



        // Tested action: local swap
        let response_result = app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            &[]
        );



        // Make sure the swap fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::InvalidAssets {}
        ));

    }


    #[test]
    #[ignore]   // TODO ! This test currently fails, as the return of the local swap is 0 (and cw20 does not allow 0-valued transferred). Is this desired?
    fn test_local_swap_zero_from_amount() {

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

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();

        // Swap amount set to 0
        let swap_amount = Uint128::zero();

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );



        //TODO currently the following fails, as a zero-valued token transfer is not allowed. Do we want this? In that case, make this test pass.
        // Tested action: local swap
        app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();

    }


    #[test]
    #[ignore]   // TODO ! This test currently fails, as the return of the local swap is 0 (and cw20 does not allow 0-valued transferred). Is this desired?
    fn test_local_swap_zero_to_amount() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD, Uint128::from(2u64), Uint128::from(3u64) * WAD];   // ! Initialize to_asset's vault balance to a very small value, to force the output of swaps to be 0
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

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];

        // Swap 10% of the pool
        let swap_amount = vault_initial_balances[from_asset_idx] * Uint128::from(10u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );

        // Check the expected swap return is 0 (make sure the test is properly configured)
        let expected_swap = compute_expected_local_swap(
            swap_amount,
            from_weight,
            from_balance,
            to_weight,
            to_balance,
            Some(DEFAULT_TEST_POOL_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );
        assert_eq!(
            expected_swap.to_amount as u64,     // Cast to u64 to ignore any decimal places
            0u64
        );



        //TODO currently the following fails, as a zero-valued token transfer is not allowed. Do we want this? In that case, make this test pass.
        // Tested action: local swap
        app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();

    }


    #[test]
    fn test_local_swap_zero_gov_fee() {
        // This test verifies that zero-valued governance fees do not cause local swaps to fail.
        // This is important, as cw20 does not allow zero-valued token transfers. Hence if a governace fee
        // transfer message is set for a zero-valued governance fee, the transaction will fail

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

        // Set the governance fee to 0 (note the default mock vault has a non-zero governance fee)
        mock_set_governance_fee_share(
            &mut app,
            vault.clone(),
            Uint64::zero()
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();

        // Swap 25% of the pool
        let swap_amount = vault_initial_balances[from_asset_idx] * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            LOCAL_SWAPPER.to_string(),
        );

        // Set vault allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(LOCAL_SWAPPER),
            vault.to_string()
        );



        // Tested action: local swap
        app.execute_contract(
            Addr::unchecked(LOCAL_SWAPPER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset: from_asset.to_string(),
                to_asset: to_asset.to_string(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            &[]
        ).unwrap();


        // Verify no governance fee was collected
        let factory_owner_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), FACTORY_OWNER.to_string());
        assert_eq!(
            factory_owner_from_asset_balance,
            Uint128::zero()
        );

    }

}