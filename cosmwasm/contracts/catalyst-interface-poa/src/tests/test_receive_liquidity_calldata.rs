mod test_receive_liquidity_calldata {

    use catalyst_interface_common::catalyst_payload::CatalystEncodedAddress;
    use catalyst_vault_common::bindings::Asset;
    use cosmwasm_std::{Uint128, Addr, Binary};
    use catalyst_types::{U256, u256};
    use test_helpers::{definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, VAULT_TOKEN_DENOM}, env::CustomTestEnv, contract::{mock_factory_deploy_vault, mock_set_vault_connection, mock_instantiate_calldata_target}, misc::{encode_payload_address, get_response_attribute}, vault_token::CustomTestVaultToken};

    use crate::tests::{TestEnv, TestVaultToken, helpers::{mock_instantiate_interface, vault_contract_storage, encode_mock_send_liquidity_packet}, parameters::{TEST_VAULT_ASSET_COUNT, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION}};
    use crate::msg::ExecuteMsg as InterfaceExecuteMsg;

    #[test]
    fn test_receive_liquidity_calldata() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = vault_contract_storage(env.get_app());
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
        let from_vault = "from_vault".to_string();
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            encode_payload_address(from_vault.as_bytes()),
            true
        );

        // Define the receive liquidity configuration
        let swap_units = u256!("500000000000000000");

        // Set up calldata
        let calldata_target = mock_instantiate_calldata_target(env.get_app());
        let encoded_calldata_target = CatalystEncodedAddress::try_encode(calldata_target.as_bytes())
            .unwrap()
            .to_binary();
        let calldata_bytes = Binary(vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54]);
        let mut calldata = Vec::new();
        calldata.extend_from_slice(encoded_calldata_target.as_slice());
        calldata.extend_from_slice(&calldata_bytes);


        let mock_packet = encode_mock_send_liquidity_packet(
            from_vault,
            vault.to_string(),
            SWAPPER_B,
            swap_units,
            U256::zero(),
            U256::zero(),
            U256::zero(),   // Used only for events
            0u32,           // Used only for events
            calldata.into()
        );



        // Tested action: receive liquidity with calldata
        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            interface.clone(),
            &InterfaceExecuteMsg::PacketReceive {
                data: mock_packet,
                channel_id: CHANNEL_ID
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the calldata target is executed
        let calldata_target_event = response.events[response.events.len()-2].clone(); // Mock target event is the second last one
        let observed_action = get_response_attribute::<String>(calldata_target_event.clone(), "action").unwrap();
        assert_eq!(
            observed_action,
            "on-catalyst-call"
        );
    
        let observed_purchased_tokens = get_response_attribute::<Uint128>(calldata_target_event.clone(), "purchased_tokens").unwrap();
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let queried_user_balance = vault_token.query_balance(env.get_app(), SWAPPER_B);
        assert_eq!(
            observed_purchased_tokens,
            queried_user_balance
        );

        let observed_data = get_response_attribute::<String>(calldata_target_event.clone(), "data").unwrap();
        assert_eq!(
            Binary::from_base64(&observed_data).unwrap(),
            calldata_bytes
        );

    }
}