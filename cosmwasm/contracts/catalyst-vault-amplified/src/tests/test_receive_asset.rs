mod test_amplified_receive_asset {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use cw_multi_test::{App, Executor};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::ContractError;
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{encode_payload_address, get_response_attribute}, token::{deploy_test_tokens, query_token_balance}, definitions::{SETUP_MASTER, CHAIN_INTERFACE, CHANNEL_ID, SWAPPER_B}, contract::{mock_factory_deploy_vault, mock_set_vault_connection, mock_instantiate_calldata_target}};

    use crate::{msg::AmplifiedExecuteMsg, tests::{helpers::{compute_expected_receive_asset, amplified_vault_contract_storage}, parameters::{AMPLIFICATION, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_receive_asset_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_tokens[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");



        // Tested action: receive asset
        let response = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance,
            AMPLIFICATION
        );

        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
    
        assert!(uint128_to_f64(observed_return) <= expected_return.to_amount * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return.to_amount * 0.999999);

        // Verify the output assets have been transferred to the swapper
        let vault_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            vault_initial_balances[to_asset_idx] - observed_return
        );

        // Verify the output assets have been received by the swapper
        let swapper_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), SWAPPER_B.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            observed_return
        );

    }


    #[test]
    fn test_receive_asset_event() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_tokens[to_asset_idx].clone();
        
        let swap_units = u256!("500000000000000000");
        let min_out = u256!("123456"); // Some random value

        let from_asset = Binary("from_asset".as_bytes().to_vec()); // Some random value
        let from_amount = u256!("987654321"); // Some random value
        let from_block_number_mod = 15u32; // Some random value



        // Tested action: receive asset
        let response = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: min_out.as_uint128(),
                from_amount,
                from_asset: from_asset.clone(),
                from_block_number_mod,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-receive-asset");

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
            Attribute::new("to_asset", to_asset)
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("units", swap_units)
        );

        //NOTE: 'to_amount' is indirectly checked on `test_receive_asset_calculation`

        assert_eq!(
            event.attributes[7],
            Attribute::new("from_amount", from_amount.to_string())
        );
        assert_eq!(
            event.attributes[8],
            Attribute::new("from_asset", from_asset.to_base64())
        );

        assert_eq!(
            event.attributes[9],
            Attribute::new("from_block_number_mod", from_block_number_mod.to_string())
        );

    }


    #[test]
    fn test_receive_asset_zero_amount() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_tokens[to_asset_idx].clone();
        
        let swap_units = U256::zero();



        // Tested action: receive asset
        let response = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
        assert!(uint128_to_f64(observed_return) == 0.);

        // Verify the vault asset balance remains unchanged
        let vault_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), vault.to_string());
        assert_eq!(
            vault_to_asset_balance,
            vault_initial_balances[to_asset_idx]
        );

        // Verify the swapper asset balance remains unchanged
        let swapper_to_asset_balance = query_token_balance(&mut app, to_asset.clone(), SWAPPER_B.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            Uint128::zero()
        );

    }



    #[test]
    fn test_receive_asset_minout() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        
        // Compute the expected return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance,
            AMPLIFICATION
        ).to_amount;

        // Set min_out_valid to be slightly smaller than the expected return
        let min_out_valid = f64_to_uint128(expected_return * 0.99).unwrap();

        // Set min_out_invalid to be slightly larger than the expected return
        let min_out_invalid = f64_to_uint128(expected_return * 1.01).unwrap();



        // Tested action 1: receive asset with min_out > expected_return fails
        let response_result = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: min_out_invalid,
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
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
        


        // Tested action 2: receive asset with min_out <= expected_return succeeds
        app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: min_out_valid,
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

    }


    #[test]
    fn test_receive_asset_not_connected_vault() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // ! Do not connect the vault with the mock source vault
        let from_vault = encode_payload_address(b"from_vault");

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let swap_units = u256!("500000000000000000");



        // Tested action: receive asset
        let response_result = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::VaultNotConnected { channel_id: err_channel_id, vault: err_vault }
                if err_channel_id == CHANNEL_ID && err_vault == from_vault
        ));

    }


    #[test]
    fn test_receive_asset_invalid_to_asset_index() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = TEST_VAULT_ASSET_COUNT;   // ! Invalid index
        let swap_units = u256!("500000000000000000");



        // Tested action: receive asset
        let response_result = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetNotFound {}
        ));

    }


    #[test]
    fn test_receive_asset_caller_not_interface() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let swap_units = u256!("500000000000000000");



        // Tested action: receive asset
        let response_result = app.execute_contract(
            Addr::unchecked("not_chain_interface"),     // ! Caller is not CHAIN_INTERFACE
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
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


    #[test]
    fn test_receive_asset_calldata() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
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
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the receive asset configuration
        let to_asset_idx = 0;
        
        let swap_units = u256!("500000000000000000");

        // Define the calldata
        let calldata_target = mock_instantiate_calldata_target(&mut app);
        let calldata = Binary(vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54]);



        // Tested action: receive asset calldata
        let response = app.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault,
                to_asset_index: to_asset_idx as u8,
                to_account: SWAPPER_B.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: Some(calldata_target.to_string()),
                calldata: Some(calldata.clone())
            },
            &[]
        ).unwrap();



        // Verify the 'calldata' target is executed
        let mock_target_event = response.events[5].clone();
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