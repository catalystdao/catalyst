mod test_amplified_send_asset {
    use cosmwasm_std::{Uint128, Addr, Binary, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_types::U256;
    use catalyst_vault_common::{ContractError, msg::TotalEscrowedAssetResponse};
    use fixed_point_math::WAD;
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128, u256_to_f64}, misc::{encode_payload_address, get_response_attribute}, token::{deploy_test_tokens, transfer_tokens, set_token_allowance, query_token_balance, mock_test_token_definitions}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, SWAPPER_A, FACTORY_OWNER}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, DEFAULT_TEST_VAULT_FEE, DEFAULT_TEST_GOV_FEE, mock_set_vault_connection}};

    use crate::{msg::AmplifiedExecuteMsg, tests::helpers::{compute_expected_send_asset, amplified_vault_contract_storage}};


    const AMPLIFICATION: Uint64 = Uint64::new(900000000000000000u64);


    //TODO check event

    #[test]
    fn test_send_asset_calculation() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SWAPPER_A),
            vault.to_string()
        );



        // Tested action: send asset
        let response = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &AmplifiedExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset: from_asset.to_string(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_send_asset(
            swap_amount,
            from_weight,
            from_balance,
            AMPLIFICATION,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        
        assert!(u256_to_f64(observed_return) / 1e18 <= expected_return.u * 1.000001);
        assert!(u256_to_f64(observed_return) / 1e18 >= expected_return.u * 0.999999);

        // Verify the input assets have been transferred from the swapper to the vault
        let swapper_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

        // Verify the input assets have been received by the vault and the governance fee has been collected
        // Note: the vault fee calculation is indirectly tested via the governance fee calculation
        let vault_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), vault.to_string());
        let factory_owner_from_asset_balance = query_token_balance(&mut app, from_asset.clone(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance + factory_owner_from_asset_balance,    // Some of the swappers balance will have gone to the factory owner (governance fee)
            vault_initial_balances[from_asset_idx] + swap_amount
        );

        assert!(uint128_to_f64(factory_owner_from_asset_balance) <= expected_return.governance_fee * 1.000001);
        assert!(uint128_to_f64(factory_owner_from_asset_balance) >= expected_return.governance_fee * 0.999999);

        // Verify the input assets are escrowed
        let queried_escrowed_total = uint128_to_f64(
            app
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(vault.clone(), &crate::msg::QueryMsg::TotalEscrowedAsset { asset: from_asset.to_string() })
            .unwrap()
            .amount
        );
        let expected_escrowed_total = uint128_to_f64(swap_amount) - expected_return.vault_fee - expected_return.governance_fee;

        assert!(queried_escrowed_total <= expected_escrowed_total * 1.000001);
        assert!(queried_escrowed_total >= expected_escrowed_total * 0.999999);
    
        // Verify the fallback account/escrow is set
        // TODO how do we compute the swapHash? Where do we get the (fromAmount - fee) from?

        // Verify interface contract gets invoked
        let invoked_interface = get_response_attribute::<String>(response.events[response.events.len()-1].clone(), "_contract_addr").unwrap();
        assert_eq!(
            Addr::unchecked(invoked_interface),
            interface
        );

    }


    //TODO this test currently fails as transferring a zero-valued amount of a token is not allowed. Do we want this?
    #[test]
    #[ignore]
    fn test_send_asset_zero_amount() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let swap_amount = Uint128::zero();

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SWAPPER_A),
            vault.to_string()
        );



        // Tested action: send asset
        app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &AmplifiedExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset: from_asset.to_string(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();

    }


    #[test]
    fn test_send_asset_not_connected_vault() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, None);
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        // ! Do not set the connection with the target vault

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_tokens[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SWAPPER_A),
            vault.to_string()
        );



        // Tested action: send asset
        let response_result = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &AmplifiedExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault.clone(),
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset: from_asset.to_string(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
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
    fn test_send_asset_from_asset_not_in_vault() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, Some(mock_test_token_definitions(SETUP_MASTER.to_string(), 4)));
        let vault_tokens = tokens[0..3].to_vec();
        let vault_initial_balances = vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()];
        let vault_weights = vec![Uint64::one(), Uint64::one(), Uint64::one()];
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

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            &mut app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset = tokens[3].clone();
        let swap_amount = Uint128::from(10000000u64);

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        transfer_tokens(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SWAPPER_A),
            vault.to_string()
        );



        // Tested action: send asset
        let response_result = app.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &AmplifiedExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset: from_asset.to_string(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::AssetNotFound {}
        ));

    }

}