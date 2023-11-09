mod test_underwrite {
    use std::str::FromStr;

    use catalyst_interface_common::{msg::{InterfaceCommonQueryMsg, UnderwriteIdentifierResponse}, state::{UNDERWRITING_COLLATERAL, UNDERWRITING_COLLATERAL_BASE, UNDERWRITE_BUFFER_BLOCKS}, ContractError, catalyst_payload::CatalystEncodedAddress};
    use catalyst_vault_common::{ContractError as VaultContractError, bindings::Asset};
    use cosmwasm_std::{Uint128, Addr, Binary, Uint64};
    use catalyst_types::{U256, u256};
    use test_helpers::{math::f64_to_uint128, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, UNDERWRITER}, env::CustomTestEnv, asset::CustomTestAsset, contract::{mock_instantiate_calldata_target, mock_factory_deploy_vault, mock_set_vault_connection}, misc::{get_response_attribute, encode_payload_address}};

    use crate::{tests::{TestEnv, TestAsset, helpers::{compute_expected_receive_asset, encode_mock_send_asset_packet, mock_instantiate_interface, vault_contract_storage}, parameters::{TEST_VAULT_ASSET_COUNT, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION}}, contract::MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS};
    use crate::msg::ExecuteMsg as InterfaceExecuteMsg;


    pub struct MockTestState {
        pub interface: Addr,
        pub vault: Addr,
        pub from_vault: String,
        pub vault_assets: Vec<TestAsset>,
        pub vault_initial_balances: Vec<Uint128>,
        pub vault_weights: Vec<Uint128>
    }
    
    impl MockTestState {
    
        pub fn initialize(
            env: &mut TestEnv
        ) -> Self {
    
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(env.get_app());
            let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
            let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
            let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
            let vault_code_id = vault_contract_storage(env.get_app());
            let vault = mock_factory_deploy_vault::<Asset, _, _>(
                env,
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
            let from_vault = "from_vault".to_string();
            mock_set_vault_connection(
                env.get_app(),
                vault.clone(),
                CHANNEL_ID.to_string(),
                encode_payload_address(from_vault.as_bytes()),
                true
            );
    
            Self {
                interface,
                vault,
                from_vault,
                vault_assets,
                vault_initial_balances,
                vault_weights,
            }
        }
    }


    #[test]
    fn test_underwrite_and_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault: _,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

        // Get the expected swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance
        );

        let underwriter_provided_funds = f64_to_uint128(
            expected_return.to_amount * 1.1
        ).unwrap();

        // Fund underwriter with assets
        to_asset.transfer(
            env.get_app(),
            underwriter_provided_funds,
            Addr::unchecked(SETUP_MASTER),
            UNDERWRITER.to_string(),
        );


        
        // Tested action: underwrite
        let response = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        ).unwrap();



        // Verify the interface event
        let expected_identifier = env.get_app()
            .wrap()
            .query_wasm_smart::<UnderwriteIdentifierResponse>(
                interface.clone(),
                &InterfaceCommonQueryMsg::UnderwriteIdentifier {
                    to_vault: vault.to_string(),
                    to_asset_ref: to_asset.alias.to_string(),
                    u: swap_units,
                    min_out: Uint128::zero(),
                    to_account: SWAPPER_B.to_string(),
                    underwrite_incentive_x16,
                    calldata: Binary::default()
                }
            )
            .unwrap()
            .identifier;

        let expected_expiry = Uint64::new(env.get_app().block_info().height)
            + MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS;

        let interface_event = response.events[4].clone();
        assert_eq!(
            interface_event.ty,
            "wasm-underwrite-swap"
        );
        assert_eq!(
            interface_event.attributes[1].value,    // identifier
            expected_identifier.to_base64()
        );
        assert_eq!(
            interface_event.attributes[2].value,    // underwriter
            UNDERWRITER.to_string()
        );
        assert_eq!(
            interface_event.attributes[3].value,    // expiry
            expected_expiry.to_string()
        );


        // Verify the vault event
        let vault_event = response.events[2].clone();
        assert_eq!(
            vault_event.ty,
            "wasm-underwrite-asset"
        );
        assert_eq!(
            vault_event.attributes[1].value,    // identifier
            expected_identifier.to_base64()
        );
        let observed_vault_return = Uint128::from_str(
            &vault_event.attributes[4].value
        ).unwrap();

        
        // Verify fund transfers
        let expected_incentive = observed_vault_return
            * Uint128::from(underwrite_incentive_x16)
            >> 16;
        
        let expected_collateral = observed_vault_return
            * UNDERWRITING_COLLATERAL
            / UNDERWRITING_COLLATERAL_BASE;

        let expected_escrowed_funds = expected_incentive + expected_collateral;
        let expected_user_output = observed_vault_return - expected_incentive;
        let expected_underwriter_refund = underwriter_provided_funds - (expected_user_output + expected_escrowed_funds);

        let queried_interface_balance = env.get_app()
            .wrap()
            .query_balance(
                interface,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;

        assert_eq!(
            queried_interface_balance,
            expected_escrowed_funds
        );

        let queried_user_balance = env.get_app()
            .wrap()
            .query_balance(
                SWAPPER_B,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;

        assert_eq!(
            queried_user_balance,
            expected_user_output
        );

        let queried_underwriter_balance = env.get_app()
            .wrap()
            .query_balance(
                UNDERWRITER,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;

        assert_eq!(
            queried_underwriter_balance,
            expected_underwriter_refund
        );

    }


    #[test]
    fn test_underwrite_min_out() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault: _,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

        // Get the expected swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance
        );

        let underwriter_provided_funds = f64_to_uint128(
            expected_return.to_amount * 1.1
        ).unwrap();

        // Fund underwriter with assets
        to_asset.transfer(
            env.get_app(),
            underwriter_provided_funds,
            Addr::unchecked(SETUP_MASTER),
            UNDERWRITER.to_string(),
        );


        
        // Tested action 1: underwrite invalid min out

        // Set min out to slightly more than the expected end user return
        let min_out = f64_to_uint128(
            expected_return.to_amount
                * (1. - underwrite_incentive_x16 as f64 / f64::powf(2., 16.))
                * 1.001
        ).unwrap();

        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out,
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            VaultContractError::ReturnInsufficient { out: _, min_out: _ }
        ));


        
        // Tested action 2: underwrite valid min out

        // Set min out to slightly less than the expected end user return
        let min_out = f64_to_uint128(
            expected_return.to_amount
                * (1. - underwrite_incentive_x16 as f64 / f64::powf(2., 16.))
                * 0.999
        ).unwrap();

        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out,
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction passes
        assert!(response_result.is_ok());
    
    }


    #[test]
    fn test_underwrite_twice() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault: _,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

        // Get the expected swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance
        );

        let underwriter_provided_funds = f64_to_uint128(
            expected_return.to_amount * 1.1
        ).unwrap();

        // Fund underwriter with assets
        to_asset.transfer(
            env.get_app(),
            underwriter_provided_funds*Uint128::new(2),
            Addr::unchecked(SETUP_MASTER),
            UNDERWRITER.to_string(),
        );

        // Underwrite a swap
        let response = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        ).unwrap();

        let _underwrite_id = Binary::from_base64(
            &response.events[4].attributes[1].value
        ).unwrap();


        
        // Tested action: underwrite the same swap again
        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SwapRecentlyUnderwritten {}  //TODO error!
        ));

    }


    #[test]
    fn test_underwrite_after_fulfill() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

        // Get the expected swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance
        );

        let underwriter_provided_funds = f64_to_uint128(
            expected_return.to_amount * 1.1
        ).unwrap();

        // Fund underwriter with assets
        to_asset.transfer(
            env.get_app(),
            underwriter_provided_funds*Uint128::new(2),
            Addr::unchecked(SETUP_MASTER),
            UNDERWRITER.to_string(),
        );

        // Underwrite a swap
        let response = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        ).unwrap();

        let _underwrite_id = Binary::from_base64(
            &response.events[4].attributes[1].value
        ).unwrap();

        // Fulfill the underwrite
        let mock_packet = encode_mock_send_asset_packet(
            from_vault,
            vault.to_string(),
            SWAPPER_B,
            swap_units,
            to_asset_idx as u8,
            U256::zero(),
            U256::zero(),
            "from_asset",
            0u32,
            underwrite_incentive_x16,
            Binary::default()
        );

        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            interface.clone(),
            &InterfaceExecuteMsg::PacketReceive {
                data: mock_packet,
                channel_id: CHANNEL_ID.to_string()
            },
            vec![],
            vec![]
        ).unwrap();


        
        // Tested action 1: underwrite the same swap again on the same block
        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SwapRecentlyUnderwritten {}
        ));


        
        // Tested action 2: underwrite the same swap again on block + BUFFER
        env.get_app().update_block(|block| {
            block.height = block.height + UNDERWRITE_BUFFER_BLOCKS.u64()
        });
        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SwapRecentlyUnderwritten {}
        ));


        
        // Tested action 3: underwrite the same swap again on block + BUFFER + 1
        env.get_app().update_block(|block| {
            block.height = block.height + 1
        });
        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction passes
        assert!(response_result.is_ok());

    }


    #[test]
    fn test_underwrite_calldata() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault: _,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

        // Define the calldata
        let calldata_target = mock_instantiate_calldata_target(env.get_app());
        let encoded_calldata_target = CatalystEncodedAddress::try_encode(calldata_target.as_bytes())
            .unwrap()
            .to_binary();
        let calldata_bytes = Binary(vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54]);
        let mut calldata = Vec::new();
        calldata.extend_from_slice(encoded_calldata_target.as_slice());
        calldata.extend_from_slice(&calldata_bytes);



        // Get the expected swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance
        );

        let underwriter_provided_funds = f64_to_uint128(
            expected_return.to_amount * 1.1
        ).unwrap();

        // Fund underwriter with assets
        to_asset.transfer(
            env.get_app(),
            underwriter_provided_funds*Uint128::new(2),
            Addr::unchecked(SETUP_MASTER),
            UNDERWRITER.to_string(),
        );


        
        // Tested action: underwrite with calldata
        let response = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: calldata.into()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        ).unwrap();



        // Verify the 'calldata' target is executed
        let calldata_target_event = response.events[response.events.len()-1].clone(); // Mock target event is the last one
        let observed_action = get_response_attribute::<String>(calldata_target_event.clone(), "action").unwrap();
        assert_eq!(
            observed_action,
            "on-catalyst-call"
        );
    
        let observed_purchased_tokens = get_response_attribute::<Uint128>(calldata_target_event.clone(), "purchased_tokens").unwrap();
        let queried_user_balance = env.get_app()
            .wrap()
            .query_balance(
                SWAPPER_B,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;
        assert_eq!(
            observed_purchased_tokens,
            queried_user_balance
        );

        let observed_data = get_response_attribute::<String>(calldata_target_event.clone(), "data").unwrap();
        assert_eq!(
            Binary::from_base64(&observed_data).unwrap(),
            calldata_bytes
        )

    }

    #[test]
    fn test_underwrite_and_check_connection() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");
        let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

        // Get the expected swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance
        );

        let underwriter_provided_funds = f64_to_uint128(
            expected_return.to_amount * 1.1
        ).unwrap();

        // Fund underwriter with assets
        to_asset.transfer(
            env.get_app(),
            underwriter_provided_funds,
            Addr::unchecked(SETUP_MASTER),
            UNDERWRITER.to_string(),
        );


        
        // Tested action 1: underwrite a swap from a non-connected vault
        let not_connected_from_vault = CatalystEncodedAddress::try_encode(b"not-a-connected-vault")
            .unwrap()
            .to_binary();
        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::UnderwriteAndCheckConnection {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: not_connected_from_vault.clone(),   // ! Set a non-connected vault as origin
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::VaultNotConnected { channel_id: err_channel_id, vault: err_vault }
                if err_channel_id == CHANNEL_ID.to_string() && err_vault == not_connected_from_vault
        ));


        
        // Tested action 2: underwrite a swap from a connected vault
        let connected_from_vault = CatalystEncodedAddress::try_encode(from_vault.as_bytes())
            .unwrap()
            .to_binary();
        let response_result = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::UnderwriteAndCheckConnection {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: connected_from_vault,   // ! Set a connected vault as origin
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        );

        // Make sure the transaction passes
        assert!(response_result.is_ok());

    }
}
