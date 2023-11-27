mod test_volatile_send_liquidity_success_failure {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedLiquidityResponse, LiquidityEscrowResponse}, state::{compute_send_liquidity_hash, INITIAL_MINT_AMOUNT}, bindings::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{encode_payload_address, get_response_attribute}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, SWAPPER_A, VAULT_TOKEN_DENOM}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_set_vault_connection}, env::CustomTestEnv, vault_token::CustomTestVaultToken};

    use crate::tests::{TestEnv, TestVaultToken, helpers::mock_incentive};
    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    struct MockTest {
        pub interface: Addr,
        pub vault: Addr,
        pub from_amount: Uint128,
        pub u: U256,
        pub to_account: Binary,
        pub block_number: u32
    }

    impl MockTest {

        pub fn initiate_mock_env(test_env: &mut TestEnv) -> Self {
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(test_env.get_app());
            let vault_assets = test_env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
            let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
            let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
            let vault_code_id = volatile_vault_contract_storage(test_env.get_app());
            let vault = mock_factory_deploy_vault::<Asset, _, _>(
                test_env,
                vault_assets,
                vault_initial_balances.clone(),
                vault_weights.clone(),
                AMPLIFICATION,
                vault_code_id,
                Some(interface.clone()),
                None,
                None
            );
    
            // Connect vault with a mock vault
            let target_vault = encode_payload_address(b"target_vault");
            mock_set_vault_connection(
                test_env.get_app(),
                vault.clone(),
                CHANNEL_ID,
                target_vault.clone(),
                true
            );
    
            // Define send liquidity configuration
            let send_percentage = 0.15;
            let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
            let to_account = encode_payload_address(SWAPPER_B.as_bytes());
    
            // Fund swapper with tokens and set vault allowance
            let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
            vault_token.transfer(
                test_env.get_app(),
                swap_amount,
                Addr::unchecked(SETUP_MASTER),
                SWAPPER_A.to_string()
            );
    
            // Execute send liquidity
            let response = test_env.execute_contract(
                Addr::unchecked(SWAPPER_A),
                vault.clone(),
                &VolatileExecuteMsg::SendLiquidity {
                    channel_id: CHANNEL_ID,
                    to_vault: target_vault,
                    to_account: to_account.clone(),
                    amount: swap_amount,
                    min_vault_tokens: U256::zero(),
                    min_reference_asset: U256::zero(),
                    fallback_account: SWAPPER_A.to_string(),
                    calldata: Binary(vec![]),
                    incentive: mock_incentive()
                },
                vec![],
                vec![]
            ).unwrap();

            let u = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
            let block_number = test_env.get_app().block_info().height as u32;

            MockTest {
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action: send liquidity ack
        test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_liquidity_hash(
            mock.to_account.as_slice(),
            mock.u,
            mock.from_amount,
            mock.block_number
        );

        let queried_escrow = test_env.get_app()
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(
                mock.vault.clone(),
                &QueryMsg::LiquidityEscrow { hash: Binary(swap_hash) }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());


        // Verify total escrowed balance is updated
        let queried_total_escrowed_balance = test_env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(
                mock.vault.clone(),
                &QueryMsg::TotalEscrowedLiquidity {}
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balance.amount,
            Uint128::zero()
        );

        // Verify the vault token supply remains unchanged
        let vault_token = TestVaultToken::load(mock.vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let vault_supply = vault_token.total_supply(test_env.get_app());
        assert_eq!(
            vault_supply,
            INITIAL_MINT_AMOUNT - mock.from_amount
        );

        // Verify vault tokens have not been received by the swapper
        let swapper_vault_token_balance = vault_token.query_balance(test_env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_vault_token_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_send_liquidity_failure() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action: send liquidity timeout
        test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_liquidity_hash(
            mock.to_account.as_slice(),
            mock.u,
            mock.from_amount,
            mock.block_number
        );

        let queried_escrow = test_env.get_app()
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(
                mock.vault.clone(),
                &QueryMsg::LiquidityEscrow { hash: Binary(swap_hash) }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());

        // Verify total escrowed balance is updated
        let queried_total_escrowed_balance = test_env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(
                mock.vault.clone(),
                &QueryMsg::TotalEscrowedLiquidity {}
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balance.amount,
            Uint128::zero()
        );

        // Verify the vault token supply
        let vault_token = TestVaultToken::load(mock.vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let vault_supply = vault_token.total_supply(test_env.get_app());
        assert_eq!(
            vault_supply,
            INITIAL_MINT_AMOUNT        // The vault balance returns to the initial vault balance
        );

        // Verify the vault tokens have been received by the swapper
        let swapper_vault_token_balance = vault_token.query_balance(test_env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_vault_token_balance,
            mock.from_amount
        );

    }


    #[test]
    fn test_send_liquidity_success_event() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action: send liquidity success
        let response = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-liquidity-success");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID.to_base64())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_account", mock.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", mock.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", mock.from_amount.to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("block_number_mod", mock.block_number.to_string())
        );

    }


    #[test]
    fn test_send_liquidity_failure_event() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action: send liquidity failure
        let response = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-liquidity-failure");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID.to_base64())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_account", mock.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", mock.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", mock.from_amount.to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("block_number_mod", mock.block_number.to_string())
        );

    }


    #[test]
    fn test_send_liquidity_no_failure_after_success() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

        // Execute send liquidity ack
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();

    

        // Tested action: send liquidity timeout
        let response_result = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        );



        // Make sure transaction fails
        let error = response_result.err().unwrap().root_cause().to_string();
        assert!(error.starts_with("type: cosmwasm_std::addresses::Addr; key: "));
        assert!(error.ends_with("not found"));

    }
    

    #[test]
    fn test_send_liquidity_no_success_after_failure() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

        // Execute send liquidity timeout
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();

    

        // Tested action: send liquidity ack
        let response_result = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        );



        // Make sure transaction fails
        let error = response_result.err().unwrap().root_cause().to_string();
        assert!(error.starts_with("type: cosmwasm_std::addresses::Addr; key: "));
        assert!(error.ends_with("not found"));

    }


    #[test]
    fn test_send_liquidity_success_invalid_caller() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action: send liquidity ack
        let response_result = test_env.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action: send liquidity timeout
        let response_result = test_env.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action 1: send liquidity ack with invalid account
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send liquidity ack with invalid units
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u + u256!("1"),                              // ! Increased units
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send liquidity ack with invalid from amount
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount + Uint128::from(1u64),      // ! Increased from amount
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send liquidity ack with invalid block number
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: 101u32                            // ! Not the original block number
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account,
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();
    }

    #[test]
    fn test_send_liquidity_failure_invalid_params() {
        

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock_env(&mut test_env);

    

        // Tested action 1: send liquidity ack with invalid account
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send liquidity ack with invalid units
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u + u256!("1"),                              // ! Increased units
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send liquidity ack with invalid from amount
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount + Uint128::from(1u64),      // ! Increased from amount
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send liquidity ack with invalid block number
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: 101u32                            // ! Not the original block number
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID,
                to_account: mock.to_account,
                u: mock.u,
                escrow_amount: mock.from_amount,
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();
        
    }

}