mod test_volatile_receive_liquidity {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT, asset::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{get_response_attribute, encode_payload_address}, token::{query_token_balance, query_token_info}, definitions::{SETUP_MASTER, CHAIN_INTERFACE, CHANNEL_ID, SWAPPER_B}, contract::{mock_factory_deploy_vault, mock_set_vault_connection, mock_instantiate_calldata_target}, env::CustomTestEnv};

    use crate::tests::TestEnv;
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{compute_expected_receive_liquidity, compute_expected_reference_asset, volatile_vault_contract_storage}, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_receive_liquidity_calculation() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive liquidity configuration        
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
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
        
        // Verify the vault tokens have been minted to the swapper
        let depositor_vault_tokens_balance = query_token_balance(env.get_app(), vault.clone(), SWAPPER_B.to_string());
        assert_eq!(
            depositor_vault_tokens_balance,
            observed_return
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(env.get_app(), vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT + observed_return
        );

    }


    #[test]
    fn test_receive_liquidity_event() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive liquidity configuration        
        let swap_units = u256!("500000000000000000");
        let from_amount = u256!("12345678");      // Some random value
        let from_block_number_mod = 15u32;         // Some random value



        // Tested action: receive liquidity
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount,
                from_block_number_mod,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-receive-liquidity");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("from_vault", from_vault.to_base64())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("to_account", SWAPPER_B.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("units", swap_units)
        );

        //NOTE: 'to_amount' is indirectly checked on `test_receive_liquidity_calculation`

        assert_eq!(
            event.attributes[6],
            Attribute::new("from_amount", from_amount.to_string())
        );
        assert_eq!(
            event.attributes[7],
            Attribute::new("from_block_number_mod", from_block_number_mod.to_string())
        );

    }


    #[test]
    fn test_receive_liquidity_zero_amount() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive liquidity configuration        
        let swap_units = U256::zero();



        // Tested action: receive liquidity
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the swap return
        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
        assert!(uint128_to_f64(observed_return) == 0.);
        
        // Verify no vault tokens have been minted to the swapper
        let depositor_vault_tokens_balance = query_token_balance(env.get_app(), vault.clone(), SWAPPER_B.to_string());
        assert_eq!(
            depositor_vault_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(env.get_app(), vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT
        );

    }



    #[test]
    fn test_receive_liquidity_min_vault_tokens() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
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
        let response_result = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: min_out_invalid,
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::ReturnInsufficient { min_out: err_min_out, out: err_out}
                if err_min_out == min_out_invalid && err_out < err_min_out
        ));
        


        // Tested action 2: receive liquidity with min_out <= expected_return succeeds
        env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: min_out_valid,
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        ).unwrap();

    }



    #[test]
    fn test_receive_liquidity_min_reference_asset() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
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
        let response_result = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: min_out_invalid,
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::ReturnInsufficient { min_out: err_min_out, out: err_out}
                if err_min_out == min_out_invalid && err_out < err_min_out
        ));
        


        // Tested action 2: receive liquidity with min_reference_asset <= expected_reference_asset_amount succeeds
        env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: min_out_valid,
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        ).unwrap();

    }


    #[test]
    fn test_receive_liquidity_not_connected_vault() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // ! Do not connect the vault with the mock source vault
        let from_vault = encode_payload_address(b"from_vault");

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response_result = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::VaultNotConnected { channel_id: err_channel_id, vault: err_vault }
                if err_channel_id == CHANNEL_ID && err_vault == from_vault
        ));

    }


    #[test]
    fn test_receive_liquidity_caller_not_interface() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response_result = env.execute_contract(
            Addr::unchecked("not_chain_interface"),     // ! Caller is not CHAIN_INTERFACE
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


    #[test]
    fn test_receive_liquidity_calldata() {

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
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive liquidity configuration        
        let swap_units = u256!("500000000000000000");

        // Define the calldata
        let calldata_target = mock_instantiate_calldata_target(env.get_app());
        let calldata = Binary(vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54]);



        // Tested action: receive liquidity calldata
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: Some(calldata_target.to_string()),
                calldata: Some(calldata.clone())
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the 'calldata' target is executed
        let mock_target_event = response.events[response.events.len()-1].clone();
        let observed_action = get_response_attribute::<String>(mock_target_event.clone(), "action").unwrap();
        assert_eq!(
            observed_action,
            "on-catalyst-call"
        );
    
        let observed_purchased_tokens = get_response_attribute::<Uint128>(mock_target_event.clone(), "purchased_tokens").unwrap();
        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
        assert_eq!(
            observed_purchased_tokens,
            observed_return
        );

        let observed_data = get_response_attribute::<String>(mock_target_event.clone(), "data").unwrap();
        assert_eq!(
            Binary::from_base64(&observed_data).unwrap(),
            calldata
        )

    }

}
