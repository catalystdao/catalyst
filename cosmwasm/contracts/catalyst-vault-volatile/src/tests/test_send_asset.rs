mod test_volatile_send_asset {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use catalyst_types::U256;
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedAssetResponse, AssetEscrowResponse}, state::compute_send_asset_hash, bindings::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128, u256_to_f64, f64_to_u256}, misc::{encode_payload_address, get_response_attribute}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, SWAPPER_A, FACTORY_OWNER, SWAPPER_C}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, DEFAULT_TEST_VAULT_FEE, DEFAULT_TEST_GOV_FEE, mock_set_vault_connection}, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::{TestEnv, helpers::mock_incentive};
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{compute_expected_send_asset, volatile_vault_contract_storage}, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_send_asset_calculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send asset
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_send_asset(
            swap_amount,
            from_weight,
            from_balance,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        
        assert!(u256_to_f64(observed_return) / 1e18 <= expected_return.u * 1.000001);
        assert!(u256_to_f64(observed_return) / 1e18 >= expected_return.u * 0.999999);

        // Verify the input assets have been transferred from the swapper to the vault
        let swapper_from_asset_balance = from_asset.query_balance(env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

        // Verify the input assets have been received by the vault and the governance fee has been collected
        // Note: the vault fee calculation is indirectly tested via the governance fee calculation
        let vault_from_asset_balance = from_asset.query_balance(env.get_app(), vault.to_string());
        let factory_owner_from_asset_balance = from_asset.query_balance(env.get_app(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance + factory_owner_from_asset_balance,    // Some of the swappers balance will have gone to the factory owner (governance fee)
            vault_initial_balances[from_asset_idx] + swap_amount
        );

        assert!(uint128_to_f64(factory_owner_from_asset_balance) <= expected_return.governance_fee * 1.000001);
        assert!(uint128_to_f64(factory_owner_from_asset_balance) >= expected_return.governance_fee * 0.999999);

        // Verify the input assets are escrowed
        let queried_escrowed_total = uint128_to_f64(
            env.get_app()
                .wrap()
                .query_wasm_smart::<TotalEscrowedAssetResponse>(
                    vault.clone(),
                    &crate::msg::QueryMsg::TotalEscrowedAsset { asset_ref: from_asset.get_asset_ref() }
                )
                .unwrap()
                .amount
        );
        let expected_escrowed_total = uint128_to_f64(swap_amount) - expected_return.vault_fee - expected_return.governance_fee;

        assert!(queried_escrowed_total <= expected_escrowed_total * 1.000001);
        assert!(queried_escrowed_total >= expected_escrowed_total * 0.999999);
    
        // Verify the fallback account/escrow is set
        let observed_fee = get_response_attribute::<Uint128>(response.events[1].clone(), "fee").unwrap();

        let expected_asset_swap_hash = compute_send_asset_hash(
            to_account.as_ref(),
            observed_return,
            swap_amount - observed_fee,
            from_asset.get_asset_ref().as_str(),
            env.get_app().block_info().height as u32
        );

        let queried_fallback_account = env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(vault.clone(), &crate::msg::QueryMsg::AssetEscrow { hash: Binary(expected_asset_swap_hash) })
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            Some(Addr::unchecked(SWAPPER_C))
        );

        // Verify interface contract gets invoked
        let invoked_interface = get_response_attribute::<String>(response.events[response.events.len()-1].clone(), "_contract_address").unwrap();
        assert_eq!(
            Addr::unchecked(invoked_interface),
            interface
        );

    }


    #[test]
    fn test_send_asset_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let min_out = f64_to_u256(12345.678).unwrap();  // Some random value
        let underwrite_incentive_x16 = 8765u16;

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send asset
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out,
                fallback_account: SWAPPER_A.to_string(),
                underwrite_incentive_x16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Check the event
        let send_asset_event = response.events[1].clone();

        assert_eq!(send_asset_event.ty, "wasm-send-asset");

        assert_eq!(
            send_asset_event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID.to_base64())
        );
        assert_eq!(
            send_asset_event.attributes[2],
            Attribute::new("to_vault", target_vault.to_string())
        );
        assert_eq!(
            send_asset_event.attributes[3],
            Attribute::new("to_account", encode_payload_address(SWAPPER_B.as_bytes()).to_base64())
        );
        assert_eq!(
            send_asset_event.attributes[4],
            Attribute::new("from_asset_ref", from_asset.get_asset_ref())
        );
        assert_eq!(
            send_asset_event.attributes[5],
            Attribute::new("to_asset_index", to_asset_idx.to_string())
        );
        assert_eq!(
            send_asset_event.attributes[6],
            Attribute::new("from_amount", swap_amount)
        );
        assert_eq!(
            send_asset_event.attributes[7],
            Attribute::new("min_out", min_out)
        );
        assert_eq!(
            send_asset_event.attributes[9],
            Attribute::new("underwrite_incentive_x16", underwrite_incentive_x16.to_string())
        );

        // NOTE: the 'units' and 'fee' fields are indirectly checked on `test_send_asset_calculation`.

    }


    #[test]
    fn test_send_asset_zero_amount() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let swap_amount = Uint128::zero();

        let to_asset_idx = 1;



        // Tested action: send asset
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Verify that 0 units are sent
        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        assert_eq!(
            observed_return,
            U256::zero()
        )

    }


    #[test]
    fn test_send_asset_not_connected_vault() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        // ! Do not set the connection with the target vault

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send asset
        let response_result = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
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

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset = env.get_assets()[TEST_VAULT_ASSET_COUNT+1].clone();
        let swap_amount = Uint128::from(10000000u64);

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send asset
        let response_result = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast::<ContractError>().unwrap(),
            ContractError::AssetNotFound {}
        ));

    }


    #[test]
    fn test_send_asset_calldata() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        // Define the calldata
        let target_account = encode_payload_address("CALLDATA_ADDRESS".as_bytes());
        let target_data = vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54];
        let calldata = Binary([target_account.0, target_data].concat());


        // Tested action: send asset calldata
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_A.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: calldata.clone(),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Verify the swap return
        let payload_calldata = Binary::from_base64(
            &get_response_attribute::<String>(response.events[response.events.len()-1].clone(), "calldata").unwrap()
        ).unwrap();

        assert_eq!(
            payload_calldata,
            calldata
        );

    }


    #[test]
    fn test_send_asset_invalid_funds() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        let other_asset = env.get_assets()[TEST_VAULT_ASSET_COUNT].clone();



        // Tested action 1: no funds
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![],   // ! Do not send funds
            vec![]
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetNotReceived { asset }
                if asset == Into::<Asset>::into(from_asset.clone()).to_string()
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "No allowance for this account".to_string()
        );



        // Tested action 2: invalid asset
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![other_asset.clone()],   // ! Send 'other_asset' instead of 'from_asset'
            vec![swap_amount]
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::AssetNotReceived { asset }
                if asset == Into::<Asset>::into(from_asset.clone()).to_string()
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "No allowance for this account".to_string()
        );



        // Tested action 3: asset amount too low
        let response_result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount - Uint128::one()]
        );

        // Make sure the transaction fails
        assert!(response_result.is_err());
        #[cfg(feature="asset_native")]
        matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::UnexpectedAssetAmountReceived { received_amount, expected_amount, asset }
                if
                    received_amount == swap_amount - Uint128::one() &&
                    expected_amount == swap_amount &&
                    asset == Into::<Asset>::into(from_asset.clone()).to_string()
        );
        #[cfg(feature="asset_cw20")]
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            format!("Cannot Sub with {} and {}", swap_amount - Uint128::one(), swap_amount)
        );



        // NOTE: Too many assets do not constitute invalid funds, as excess assets sent are used
        // to pay for the relaying incentive.



        // Make sure the swap works for a valid amount
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();

    }


    #[test]
    fn test_send_asset_fixed_units_calculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        // Get the expected return of a normal 'send_asset'
        let expected_return = compute_expected_send_asset(
            swap_amount,
            from_weight,
            from_balance,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        // Set the 'fixed units' to a value slightly smaller than the expected swap output
        let fixed_units = f64_to_u256(expected_return.u * 1e18 * 0.99999).unwrap();



        // Tested action: send asset
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAssetFixedUnits {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                u: fixed_units,
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        
        assert_eq!(
            observed_return,
            fixed_units
        );

        // Verify the input assets have been transferred from the swapper to the vault
        let swapper_from_asset_balance = from_asset.query_balance(env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_send_asset_fixed_units_too_few_units() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
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
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = vault_assets[from_asset_idx].clone();
        let from_weight = vault_weights[from_asset_idx];
        let from_balance = vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens and set vault allowance
        from_asset.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        // Get the expected return of a normal 'send_asset'
        let expected_return = compute_expected_send_asset(
            swap_amount,
            from_weight,
            from_balance,
            Some(DEFAULT_TEST_VAULT_FEE),
            Some(DEFAULT_TEST_GOV_FEE)
        );

        // Set the 'fixed units' to a value slightly larger than the expected swap output
        let fixed_units = f64_to_u256(expected_return.u * 1e18 * 1.00001).unwrap();



        // Tested action: send asset
        let response_result = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendAssetFixedUnits {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                u: fixed_units,
                fallback_account: SWAPPER_C.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        );



        // Verify the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::ReturnInsufficientUnits { units: err_units, fixed_units: err_fixed_units }
                if err_units < err_fixed_units && err_fixed_units == fixed_units
        ));

    }

}