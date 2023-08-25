mod test_amplified_send_liquidity {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use cw_multi_test::{App, Executor};
    use catalyst_types::{U256, I256, u256};
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedLiquidityResponse, LiquidityEscrowResponse}, state::{INITIAL_MINT_AMOUNT, compute_send_liquidity_hash}};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128, u256_to_f64}, misc::{encode_payload_address, get_response_attribute}, token::{deploy_test_tokens, transfer_tokens, query_token_balance, query_token_info}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_A, SWAPPER_B, SWAPPER_C}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_set_vault_connection}};

    use crate::{msg::AmplifiedExecuteMsg, tests::{helpers::{compute_expected_send_liquidity, amplified_vault_contract_storage}, parameters::{AMPLIFICATION, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_send_liquidity_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
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
            Some(interface.clone()),
            None
        );

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

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
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_send_liquidity(
            swap_amount,
            vault_weights.clone(),
            vault_initial_balances,
            INITIAL_MINT_AMOUNT,
            I256::zero(),           // Unit tracker: should be at 0, since no swaps have been executed yet
            AMPLIFICATION
        );

        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        
        assert!(u256_to_f64(observed_return) / 1e18 <= expected_return.u * 1.000001);
        assert!(u256_to_f64(observed_return) / 1e18 >= expected_return.u * 0.999999);


        // Verify the vault tokens have been burnt
        let swapper_vault_tokens_balance = query_token_balance(&mut app, vault.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_vault_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT - swap_amount
        );

        // Verify the vault tokens are escrowed
        let queried_escrowed_total = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(vault.clone(), &crate::msg::QueryMsg::TotalEscrowedLiquidity {  })
            .unwrap()
            .amount;

        assert!(queried_escrowed_total == swap_amount);
    
        // Verify the fallback account/escrow is set
        let expected_liquidity_swap_hash = compute_send_liquidity_hash(
            to_account.as_ref(),
            observed_return,
            swap_amount,
            app.block_info().height as u32
        );

        let queried_fallback_account = app
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(vault.clone(), &crate::msg::QueryMsg::LiquidityEscrow { hash: Binary(expected_liquidity_swap_hash) })
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


    #[test]
    fn test_send_liquidity_event() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
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
            Some(interface.clone()),
            None
        );

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());
        let min_vault_tokens = u256!("123456789");  // Some random value
        let min_reference_asset = u256!("987654321");  // Some random value

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
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault.clone(),
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens,
                min_reference_asset,
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-liquidity");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_vault", target_vault.to_base64())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("to_account", to_account.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("from_amount", swap_amount)
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("min_vault_tokens", min_vault_tokens)
        );
        assert_eq!(
            event.attributes[6],
            Attribute::new("min_reference_asset", min_reference_asset)
        );

        //NOTE: 'units' is indirectly checked on `test_send_liquidity_calculation`

    }


    #[test]
    fn test_send_liquidity_zero_amount() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
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
            Some(interface.clone()),
            None
        );

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
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
        let response = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();



        // Verify that 0 units are sent
        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        assert_eq!(
            observed_return,
            U256::zero()
        )

    }


    #[test]
    fn test_send_liquidity_not_connected_vault() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
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
            Some(interface.clone()),
            None
        );

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        // ! Do not set the connection with the target vault

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
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault.clone(),
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        );
    


        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::VaultNotConnected { channel_id: err_channel_id, vault: err_vault }
                if err_channel_id == CHANNEL_ID && err_vault == target_vault
        ));

    }
    

    #[test]
    fn test_send_liquidity_calldata() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
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
            Some(interface.clone()),
            None
        );

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens
        transfer_tokens(
            &mut app,
            swap_amount,
            vault.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        // Define the calldata
        let target_account = encode_payload_address("CALLDATA_ADDRESS".as_bytes());
        let target_data = vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54];
        let calldata = Binary([target_account.0, target_data].concat());



        // Tested action: send liquidity calldata
        let response = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: calldata.clone()
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let payload_calldata = Binary::from_base64(
            &get_response_attribute::<String>(response.events[4].clone(), "calldata").unwrap()
        ).unwrap();

        assert_eq!(
            payload_calldata,
            calldata
        );

    }

}