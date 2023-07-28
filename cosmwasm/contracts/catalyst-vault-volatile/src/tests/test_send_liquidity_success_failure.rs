mod test_volatile_send_liquidity_success_failure {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use cw_multi_test::{App, Executor};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedLiquidityResponse, LiquidityEscrowResponse}, state::{compute_send_liquidity_hash, INITIAL_MINT_AMOUNT}};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{encode_payload_address, get_response_attribute}, token::{deploy_test_tokens, transfer_tokens, query_token_info, query_token_balance}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, SWAPPER_A}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_set_vault_connection}};

    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    struct TestEnv {
        pub interface: Addr,
        pub vault: Addr,
        pub from_amount: Uint128,
        pub u: U256,
        pub to_account: Binary,
        pub block_number: u32
    }

    impl TestEnv {

        pub fn initiate_mock_env(app: &mut App) -> Self {
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(app);
            let vault_assets = deploy_test_tokens(app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
            let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
            let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
            let vault_code_id = volatile_vault_contract_storage(app);
            let vault = mock_factory_deploy_vault(
                app,
                vault_assets.iter().map(|token_addr| token_addr.to_string()).collect(),
                vault_initial_balances.clone(),
                vault_weights.clone(),
                AMPLIFICATION,
                vault_code_id,
                Some(interface.clone()),
                None
            );
    
            // Connect vault with a mock vault
            let target_vault = encode_payload_address(b"target_vault");
            mock_set_vault_connection(
                app,
                vault.clone(),
                CHANNEL_ID.to_string(),
                target_vault.clone(),
                true
            );
    
            // Define send liquidity configuration
            let send_percentage = 0.15;
            let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
            let to_account = encode_payload_address(SWAPPER_B.as_bytes());
    
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
                    to_vault: target_vault,
                    to_account: to_account.clone(),
                    amount: swap_amount,
                    min_vault_tokens: U256::zero(),
                    min_reference_asset: U256::zero(),
                    fallback_account: SWAPPER_A.to_string(),
                    calldata: Binary(vec![])
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
                to_account,
                block_number
            }
    
        }

    }


    #[test]
    fn test_send_liquidity_success() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity ack
        app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
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
                &QueryMsg::LiquidityEscrow { hash: Binary(swap_hash) }
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
    fn test_send_liquidity_failure() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity timeout
        app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
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
                &QueryMsg::LiquidityEscrow { hash: Binary(swap_hash) }
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
    fn test_send_liquidity_success_event() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity success
        let response = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-liquidity-success");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_account", env.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", env.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", env.from_amount.to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("block_number_mod", env.block_number.to_string())
        );

    }


    #[test]
    fn test_send_liquidity_failure_event() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity failure
        let response = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-liquidity-failure");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_account", env.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", env.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", env.from_amount.to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("block_number_mod", env.block_number.to_string())
        );

    }


    #[test]
    fn test_send_liquidity_no_failure_after_success() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

        // Execute send liquidity ack
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();

    

        // Tested action: send liquidity timeout
        let response_result = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure transaction fails
        assert!(
            format!(
                "{}", response_result.err().unwrap().root_cause()
            ).contains("Addr not found")
        )

    }
    

    #[test]
    fn test_send_liquidity_no_success_after_failure() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

        // Execute send liquidity timeout
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();

    

        // Tested action: send liquidity ack
        let response_result = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure transaction fails
        assert!(
            format!(
                "{}", response_result.err().unwrap().root_cause()
            ).contains("Addr not found")
        )

    }


    #[test]
    fn test_send_liquidity_success_invalid_caller() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity ack
        let response_result = app.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
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
    fn test_send_liquidity_failure_invalid_caller() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send liquidity timeout
        let response_result = app.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
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
    fn test_send_liquidity_success_invalid_params() {
        

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action 1: send liquidity ack with invalid account
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send liquidity ack with invalid units
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u + u256!("1"),                              // ! Increased units
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send liquidity ack with invalid from amount
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount + Uint128::from(1u64),      // ! Increased from amount
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send liquidity ack with invalid block number
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: 101u32                            // ! Not the original block number
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account,
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();
    }

    #[test]
    fn test_send_liquidity_failure_invalid_params() {
        

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action 1: send liquidity ack with invalid account
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send liquidity ack with invalid units
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u + u256!("1"),                              // ! Increased units
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send liquidity ack with invalid from amount
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount + Uint128::from(1u64),      // ! Increased from amount
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send liquidity ack with invalid block number
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: 101u32                            // ! Not the original block number
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account,
                u: env.u,
                escrow_amount: env.from_amount,
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();
        
    }

}