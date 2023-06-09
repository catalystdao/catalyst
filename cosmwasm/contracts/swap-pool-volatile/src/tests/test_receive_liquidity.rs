mod test_volatile_receive_liquidity {
    use cosmwasm_std::{Uint128, Addr, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_types::{U256, u256};
    use swap_pool_common::{ContractError, state::INITIAL_MINT_AMOUNT};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{deploy_test_tokens, WAD, query_token_balance, get_response_attribute, mock_set_pool_connection, CHANNEL_ID, SWAPPER_B, CHAIN_INTERFACE, compute_expected_receive_liquidity, query_token_info, mock_factory_deploy_vault, compute_expected_reference_asset, encode_payload_address}, math_helpers::{uint128_to_f64, f64_to_uint128}}};

    //TODO check event

    #[test]
    fn test_receive_liquidity_calculation() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect pool with a mock pool
        let from_pool = encode_payload_address(b"from_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_pool.clone(),
            true
        );

        // Define the receive liquidity configuration        
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_receive_liquidity(
            swap_units,
            vault_weights.clone(),
            INITIAL_MINT_AMOUNT
        );

        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
    
        assert!(uint128_to_f64(observed_return) <= expected_return.to_amount * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return.to_amount * 0.999999);
        
        // Verify the pool tokens have been minted to the swapper
        let depositor_pool_tokens_balance = query_token_balance(&mut app, vault.clone(), SWAPPER_B.to_string());
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


    //TODO this test currently fails as minting a zero-valued amount of a token is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_receive_liquidity_zero_amount() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect pool with a mock pool
        let from_pool = encode_payload_address(b"from_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_pool.clone(),
            true
        );

        // Define the receive liquidity configuration        
        let swap_units = U256::zero();



        // Tested action: receive liquidity
        let response = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
        assert!(uint128_to_f64(observed_return) == 0.);
        
        // Verify no pool tokens have been minted to the swapper
        let depositor_pool_tokens_balance = query_token_balance(&mut app, vault.clone(), SWAPPER_B.to_string());
        assert_eq!(
            depositor_pool_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total pool tokens supply
        let pool_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            pool_token_info.total_supply,
            INITIAL_MINT_AMOUNT
        );

    }



    #[test]
    fn test_receive_liquidity_min_pool_tokens() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect pool with a mock pool
        let from_pool = encode_payload_address(b"from_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_pool.clone(),
            true
        );

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");
        
        // Compute the expected return
        let expected_return = compute_expected_receive_liquidity(
            swap_units,
            vault_weights.clone(),
             INITIAL_MINT_AMOUNT
        ).to_amount;

        // Set min_out_valid to be slightly smaller than the expected return
        let min_out_valid = f64_to_uint128(expected_return * 0.99).unwrap();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = f64_to_uint128(expected_return * 1.01).unwrap();



        // Tested action 1: receive liquidity with min_out > expected_return fails
        let response_result = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool: from_pool.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: min_out_invalid,
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::ReturnInsufficient { min_out: err_min_out, out: err_out}
                if err_min_out == min_out_invalid && err_out < err_min_out
        ));
        


        // Tested action 2: receive liquidity with min_out <= expected_return succeeds
        app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: min_out_valid,
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

    }



    #[test]
    fn test_receive_liquidity_min_reference_asset() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect pool with a mock pool
        let from_pool = encode_payload_address(b"from_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_pool.clone(),
            true
        );

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");
        
        // Compute the expected return and the expected reference asset value
        let expected_return = compute_expected_receive_liquidity(
            swap_units,
            vault_weights.clone(),
             INITIAL_MINT_AMOUNT
        ).to_amount;

        let expected_reference_asset_amount = compute_expected_reference_asset(
            f64_to_uint128(expected_return).unwrap(),
            vault_initial_balances,
            vault_weights,
            INITIAL_MINT_AMOUNT,
            Uint128::zero()
        ).amount;

        // Set min_out_valid to be slightly smaller than the expected reference asset value
        let min_out_valid = f64_to_uint128(expected_reference_asset_amount * 0.99).unwrap();

        // Set min_out_invalid to be slightly larger than the expected reference asset value
        let min_out_invalid = f64_to_uint128(expected_reference_asset_amount * 1.01).unwrap();



        // Tested action 1: receive liquidity with min_reference_asset > expected_reference_asset_amount fails
        let response_result = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool: from_pool.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: Uint128::zero(),
                min_reference_asset: min_out_invalid,
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::ReturnInsufficient { min_out: err_min_out, out: err_out}
                if err_min_out == min_out_invalid && err_out < err_min_out
        ));
        


        // Tested action 2: receive liquidity with min_reference_asset <= expected_reference_asset_amount succeeds
        app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: Uint128::zero(),
                min_reference_asset: min_out_valid,
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

    }


    #[test]
    fn test_receive_liquidity_not_connected_pool() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // ! Do not connect the pool with the mock source pool
        let from_pool = encode_payload_address(b"from_pool");

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response_result = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool: from_pool.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::PoolNotConnected { channel_id: err_channel_id, pool: err_pool }
                if err_channel_id == CHANNEL_ID && err_pool == from_pool
        ));

    }


    #[test]
    fn test_receive_liquidity_caller_not_interface() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect pool with a mock pool
        let from_pool = encode_payload_address(b"from_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_pool.clone(),
            true
        );

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response_result = app.execute_contract(
            Addr::unchecked("not_chain_interface"),     // ! Caller is not CHAIN_INTERFACE
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_pool,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_pool_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}
