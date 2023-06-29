mod test_volatile_receive_liquidity {
    use cosmwasm_std::{Uint128, Addr, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, state::INITIAL_MINT_AMOUNT};
    use fixed_point_math::WAD;
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{get_response_attribute, encode_payload_address}, token::{deploy_test_tokens, query_token_balance, query_token_info}, definitions::{SETUP_MASTER, CHAIN_INTERFACE, CHANNEL_ID, SWAPPER_B}, contract::{mock_factory_deploy_vault, mock_set_vault_connection}};

    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{compute_expected_receive_liquidity, compute_expected_reference_asset, volatile_vault_contract_storage}}};

    //TODO check event

    #[test]
    fn test_receive_liquidity_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
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

        // Define the receive liquidity configuration        
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response = app.execute_contract(
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
        
        // Verify the vault tokens have been minted to the swapper
        let depositor_vault_tokens_balance = query_token_balance(&mut app, vault.clone(), SWAPPER_B.to_string());
        assert_eq!(
            depositor_vault_tokens_balance,
            observed_return
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT + observed_return
        );

    }


    //TODO this test currently fails as minting a zero-valued amount of a token is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_receive_liquidity_zero_amount() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
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

        // Define the receive liquidity configuration        
        let swap_units = U256::zero();



        // Tested action: receive liquidity
        let response = app.execute_contract(
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
            &[]
        ).unwrap();



        // Verify the swap return
        let observed_return = get_response_attribute::<Uint128>(response.events[1].clone(), "to_amount").unwrap();
        assert!(uint128_to_f64(observed_return) == 0.);
        
        // Verify no vault tokens have been minted to the swapper
        let depositor_vault_tokens_balance = query_token_balance(&mut app, vault.clone(), SWAPPER_B.to_string());
        assert_eq!(
            depositor_vault_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_info = query_token_info(&mut app, vault.clone());
        assert_eq!(
            vault_token_info.total_supply,
            INITIAL_MINT_AMOUNT
        );

    }



    #[test]
    fn test_receive_liquidity_min_vault_tokens() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
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
            &[]
        ).unwrap();

    }



    #[test]
    fn test_receive_liquidity_min_reference_asset() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
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
            &[]
        ).unwrap();

    }


    #[test]
    fn test_receive_liquidity_not_connected_vault() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None
        );

        // ! Do not connect the vault with the mock source vault
        let from_vault = encode_payload_address(b"from_vault");

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response_result = app.execute_contract(
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
    fn test_receive_liquidity_caller_not_interface() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
        let vault_code_id = volatile_vault_contract_storage(&mut app);
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
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

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");



        // Tested action: receive liquidity
        let response_result = app.execute_contract(
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
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}
