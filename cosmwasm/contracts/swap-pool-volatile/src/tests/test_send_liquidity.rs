mod test_volatile_send_liquidity {
    use cosmwasm_std::{Uint128, Addr};
    use cw_multi_test::{App, Executor};
    use ethnum::U256;
    use swap_pool_common::{ContractError, msg::{TotalEscrowedLiquidityResponse, LiquidityEscrowResponse}, state::{INITIAL_MINT_AMOUNT, compute_send_liquidity_hash}};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{mock_instantiate, SETUP_MASTER, deploy_test_tokens, WAD, mock_initialize_pool, query_token_balance, transfer_tokens, get_response_attribute, mock_set_pool_connection, CHANNEL_ID, SWAPPER_B, SWAPPER_A, mock_instantiate_interface, compute_expected_send_liquidity, query_token_info, SWAPPER_C}, math_helpers::{uint128_to_f64, f64_to_uint128, u256_to_f64}}};

    //TODO check event

    #[test]
    fn test_send_liquidity_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault = mock_instantiate(&mut app, Some(interface.clone()));
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        let vault_config = mock_initialize_pool(
            &mut app,
            vault.clone(),
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD],
            vec![1u64, 1u64, 1u64]
        );

        // Set target mock pool
        let target_pool = Addr::unchecked("target_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_pool.as_bytes().to_vec(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send liquidity
        let response = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_pool: target_pool.as_bytes().to_vec(),
                to_account: SWAPPER_B.as_bytes().to_vec(),
                amount: swap_amount,
                min_out: U256::ZERO,
                fallback_account: SWAPPER_C.to_string(),
                calldata: vec![]
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_send_liquidity(
            swap_amount,
            vault_config.weights.clone(),
            INITIAL_MINT_AMOUNT,
        );

        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        
        assert!(u256_to_f64(observed_return) / 1e18 <= expected_return.u * 1.000001);
        assert!(u256_to_f64(observed_return) / 1e18 >= expected_return.u * 0.999999);


        // Verify the pool tokens have been burnt
        let swapper_pool_tokens_balance = query_token_balance(&mut app, vault.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_pool_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total pool tokens supply
        let pool_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            pool_token_info.total_supply,
            INITIAL_MINT_AMOUNT - swap_amount
        );

        // Verify the pool tokens are escrowed
        let queried_escrowed_total = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(vault.clone(), &crate::msg::QueryMsg::TotalEscrowedLiquidity {  })
            .unwrap()
            .amount;

        assert!(queried_escrowed_total == swap_amount);
    
        // Verify the fallback account/escrow is set
        let expected_liquidity_swap_hash = compute_send_liquidity_hash(
            SWAPPER_B.as_bytes(),
            observed_return,
            swap_amount,
            app.block_info().height as u32
        );

        let queried_fallback_account = app
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(vault.clone(), &crate::msg::QueryMsg::LiquidityEscrow { hash: expected_liquidity_swap_hash })
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            Some(Addr::unchecked(SWAPPER_C))
        );
        

        // Verify interface contract gets invoked
        let invoked_interface = get_response_attribute::<String>(response.events[response.events.len()-1].clone(), "_contract_addr").unwrap();
        assert_eq!(
            Addr::unchecked(invoked_interface),
            interface
        );

    }


    //TODO this test currently fails as burning a zero-valued amount of a token is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_send_liquidity_zero_amount() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault = mock_instantiate(&mut app, Some(interface.clone()));
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        mock_initialize_pool(
            &mut app,
            vault.clone(),
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD],
            vec![1u64, 1u64, 1u64]
        );

        // Set target mock pool
        let target_pool = Addr::unchecked("target_pool");
        mock_set_pool_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_pool.as_bytes().to_vec(),
            true
        );

        // Define send liquidity configuration
        let swap_amount = Uint128::zero();

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send liquidity
        app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_pool: target_pool.as_bytes().to_vec(),
                to_account: SWAPPER_B.as_bytes().to_vec(),
                amount: swap_amount,
                min_out: U256::ZERO,
                fallback_account: SWAPPER_C.to_string(),
                calldata: vec![]
            },
            &[]
        ).unwrap();

    }


    #[test]
    fn test_send_liquidity_not_connected_pool() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault = mock_instantiate(&mut app, Some(interface.clone()));
        let vault_tokens = deploy_test_tokens(&mut app, None, None);
        mock_initialize_pool(
            &mut app,
            vault.clone(),
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD],
            vec![1u64, 1u64, 1u64]
        );

        // Set target mock pool
        let target_pool = Addr::unchecked("target_pool");
        // ! Do not set the connection with the target pool

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send liquidity
        let response_result = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_pool: target_pool.as_bytes().to_vec(),
                to_account: SWAPPER_B.as_bytes().to_vec(),
                amount: swap_amount,
                min_out: U256::ZERO,
                fallback_account: SWAPPER_C.to_string(),
                calldata: vec![]
            },
            &[]
        );
    


        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::PoolNotConnected { channel_id: err_channel_id, pool: err_pool }
                if err_channel_id == CHANNEL_ID && err_pool == target_pool.as_bytes().to_vec()
        ));

    }

}