mod test_volatile_local_swap {
    use cosmwasm_std::{Uint128, Addr};
    use cw_multi_test::{App, Executor};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{mock_instantiate, SETUP_MASTER_ADDR, deploy_test_tokens, WAD, mock_initialize_pool, set_token_allowance, compute_expected_swap, DEFAULT_TEST_POOL_FEE, DEFAULT_TEST_GOV_FEE, query_token_balance, transfer_tokens, LOCAL_SWAPPER, FACTORY_OWNER_ADDR}, math_helpers::uint128_to_f64}};


    //TODO add test for the local swap event


    #[test]
    fn test_local_swap_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault = mock_instantiate(&mut app, false);
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_config = mock_initialize_pool(
            &mut app,
            vault.clone(),
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD],
            vec![1u64, 1u64, 1u64]
        );

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let from_weight = vault_config.weights[from_asset_idx];
        let from_balance = vault_config.assets_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = vault_tokens[to_asset_idx].clone();
        let to_weight = vault_config.weights[from_asset_idx];
        let to_balance = vault_config.assets_balances[to_asset_idx];

        // Swap 25% of the pool
        let swap_amount = vault_config.assets_balances[from_asset_idx] * Uint128::from(25u64)/ Uint128::from(100u64);

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER_ADDR),
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
        let expected_swap = compute_expected_swap(
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
        let factory_owner_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), FACTORY_OWNER_ADDR.to_string());
        assert_eq!(
            vault_from_asset_balance + factory_owner_from_asset_balance,    // Some of the swappers balance will have gone to the factory owner (governance fee)
            vault_config.assets_balances[from_asset_idx] + swap_amount
        );

        assert!(uint128_to_f64(factory_owner_from_asset_balance) <= expected_swap.governance_fee * 1.000001);
        assert!(uint128_to_f64(factory_owner_from_asset_balance) >= expected_swap.governance_fee * 0.999999);

        // Verify the output assets have been transferred to the swapper
        let vault_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            vault_config.assets_balances[to_asset_idx] - observed_return
        );

        // Verify the output assets have been received by the swapper
        let swapper_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), LOCAL_SWAPPER.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            observed_return
        );

    }


    // Pool/Governance fee calculation + gov fee is transferred

    // From asset not in pool

    // To asset not in pool

    // Min out not fulfilled

    // From swap_amount 0

    // To swap_amount 0

    // Gov fee 0
}