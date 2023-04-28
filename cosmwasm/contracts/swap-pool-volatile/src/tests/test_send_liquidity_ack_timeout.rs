mod test_volatile_send_liquidity_ack_timeout {
    use cosmwasm_std::{Uint128, Addr};
    use cw_multi_test::{App, Executor};
    use ethnum::{U256, uint};
    use swap_pool_common::{ContractError, msg::{TotalEscrowedLiquidityResponse, LiquidityEscrowResponse}, state::{compute_send_liquidity_hash, INITIAL_MINT_AMOUNT}};

    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::{helpers::{mock_instantiate, SETUP_MASTER, deploy_test_tokens, WAD, mock_initialize_pool, query_token_balance, transfer_tokens, get_response_attribute, mock_set_pool_connection, CHANNEL_ID, SWAPPER_B, SWAPPER_A, mock_instantiate_interface, query_token_info}, math_helpers::{uint128_to_f64, f64_to_uint128}}};

    //TODO check events

    struct TestEnv {
        pub interface: Addr,
        pub vault: Addr,
        pub from_amount: Uint128,
        pub u: U256,
        pub to_account: Vec<u8> ,
        pub block_number: u32
    }

    impl TestEnv {

        pub fn initiate_mock_env(app: &mut App) -> Self {
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(app);
            let vault = mock_instantiate(app, Some(interface.clone()));
            let vault_tokens = deploy_test_tokens(app, None, None);
            mock_initialize_pool(
                app,
                vault.clone(),
                vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
                vec![Uint128::from(1u64) * WAD, Uint128::from(2u64) * WAD, Uint128::from(3u64) * WAD],
                vec![1u64, 1u64, 1u64]
            );
    
            // Connect pool with a mock pool
            let target_pool = Addr::unchecked("target_pool");
            mock_set_pool_connection(
                app,
                vault.clone(),
                CHANNEL_ID.to_string(),
                target_pool.as_bytes().to_vec(),
                true
            );
    
            // Define send liquidity configuration
            let send_percentage = 0.15;
            let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
    
            // Fund swapper with tokens and set vault allowance
            transfer_tokens(
                app,
                swap_amount,
                vault.clone(),
                Addr::unchecked(SETUP_MASTER),
                SWAPPER_A.to_string()
            );
    
            // Execute send liquidity
            let response = app.execute_contract(
                Addr::unchecked(SWAPPER_A),
                vault.clone(),
                &VolatileExecuteMsg::SendLiquidity {
                    channel_id: CHANNEL_ID.to_string(),
                    to_pool: target_pool.as_bytes().to_vec(),
                    to_account: SWAPPER_B.as_bytes().to_vec(),
                    amount: swap_amount,
                    min_out: U256::ZERO,
                    fallback_account: SWAPPER_A.to_string(),
                    calldata: vec![]
                },
                &[]
            ).unwrap();

            let u = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
            let block_number = app.block_info().height as u32;

            TestEnv {
                interface,
                vault,
                from_amount: swap_amount,
                u,
                to_account: SWAPPER_B.as_bytes().to_vec(),
                block_number
            }
    
        }

    }


    #[test]
    fn test_send_liquidity_ack() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity ack
        app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_liquidity_hash(
            env.to_account.as_slice(),
            env.u,
            env.from_amount,
            env.block_number
        );

        let queried_escrow = app
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(
                env.vault.clone(),
                &QueryMsg::LiquidityEscrow { hash: swap_hash }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());


        // Verify total escrowed balance is updated
        let queried_total_escrowed_balance = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(
                env.vault.clone(),
                &QueryMsg::TotalEscrowedLiquidity {}
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balance.amount,
            Uint128::zero()
        );

        // Verify the vault token supply remains unchanged
        let vault_supply = query_token_info(&mut app, env.vault.clone()).total_supply;
        assert_eq!(
            vault_supply,
            INITIAL_MINT_AMOUNT - env.from_amount
        );

        // Verify vault tokens have not been received by the swapper
        let swapper_vault_token_balance = query_token_balance(&mut app, env.vault.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_vault_token_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_send_liquidity_timeout() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity timeout
        app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_liquidity_hash(
            env.to_account.as_slice(),
            env.u,
            env.from_amount,
            env.block_number
        );

        let queried_escrow = app
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(
                env.vault.clone(),
                &QueryMsg::LiquidityEscrow { hash: swap_hash }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());

        // Verify total escrowed balance is updated
        let queried_total_escrowed_balance = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(
                env.vault.clone(),
                &QueryMsg::TotalEscrowedLiquidity {}
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balance.amount,
            Uint128::zero()
        );

        // Verify the vault token supply
        let vault_supply = query_token_info(&mut app, env.vault.clone()).total_supply;
        assert_eq!(
            vault_supply,
            INITIAL_MINT_AMOUNT        // The vault balance returns to the initial vault balance
        );

        // Verify the vault tokens have been received by the swapper
        let swapper_vault_token_balance = query_token_balance(&mut app, env.vault.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_vault_token_balance,
            env.from_amount
        );

    }


    #[test]
    fn test_send_liquidity_no_timeout_after_ack() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

        // Execute send liquidity ack
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();

    

        // Tested action: send liquidity timeout
        let response_result = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure transaction fails
        assert!(
            format!(
                "{}", response_result.err().unwrap().root_cause()
            ).contains("Addr not found")     // TODO implement a better error rather than the current 'cosmwasm_std::addresses::Addr not found'
        )

    }
    

    #[test]
    fn test_send_liquidity_no_ack_after_timeout() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

        // Execute send liquidity timeout
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();

    

        // Tested action: send liquidity ack
        let response_result = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure transaction fails
        assert!(
            format!(
                "{}", response_result.err().unwrap().root_cause()
            ).contains("Addr not found")     // TODO implement a better error rather than the current 'cosmwasm_std::addresses::Addr not found'
        )

    }


    #[test]
    fn test_send_liquidity_ack_invalid_caller() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity ack
        let response_result = app.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))
    }


    #[test]
    fn test_send_liquidity_timeout_invalid_caller() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity timeout
        let response_result = app.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))
    }


    #[test]
    fn test_send_liquidity_ack_invalid_params() {
        

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action 1: send liquidity ack with invalid account
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: "not_to_account".as_bytes().to_vec(),   // ! Not the chain interface
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send liquidity ack with invalid units
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u + uint!("1"),                              // ! Increased units
                amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send liquidity ack with invalid from amount
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount + Uint128::from(1u64),      // ! Increased from amount
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send liquidity ack with invalid block number
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: 101u32                            // ! Not the original block number
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityAck {
                to_account: env.to_account,
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();
    }

    #[test]
    fn test_send_liquidity_timeout_invalid_params() {
        

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action 1: send liquidity ack with invalid account
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: "not_to_account".as_bytes().to_vec(),   // ! Not the chain interface
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send liquidity ack with invalid units
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u + uint!("1"),                              // ! Increased units
                amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send liquidity ack with invalid from amount
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount + Uint128::from(1u64),      // ! Increased from amount
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send liquidity ack with invalid block number
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account.clone(),
                u: env.u,
                amount: env.from_amount,
                block_number_mod: 101u32                            // ! Not the original block number
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::SendLiquidityTimeout {
                to_account: env.to_account,
                u: env.u,
                amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();
        
    }

}