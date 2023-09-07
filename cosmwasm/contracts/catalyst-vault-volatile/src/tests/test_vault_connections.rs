mod test_volatile_vault_connections {
    use cosmwasm_std::{Addr, Binary, Attribute};
    use catalyst_vault_common::{ContractError, msg::VaultConnectionStateResponse, bindings::Asset};
    use test_helpers::{misc::encode_payload_address, definitions::{SETUP_MASTER, FACTORY_OWNER}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_finish_vault_setup}, env::CustomTestEnv};

    use crate::tests::TestEnv;
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};

    fn deploy_mock_vault(env: &mut TestEnv) -> Addr {
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        mock_factory_deploy_vault::<Asset, _, _>(
            env,
            vault_assets,
            TEST_VAULT_BALANCES.to_vec(),
            TEST_VAULT_WEIGHTS.to_vec(),
            AMPLIFICATION,
            vault_code_id,
            Some(interface.clone()),
            None
        )
    }

    #[test]
    fn test_set_connection() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Deploy vault
        let vault = deploy_mock_vault(&mut env);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");



        // Tested action: set connection
        let response = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-connection");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", channel_id.to_string())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("vault", target_vault.to_base64())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("state", true.to_string())
        );

        // Verify the connection is set
        let queried_connection_state: bool = env.get_app().wrap().query_wasm_smart::<VaultConnectionStateResponse>(
            vault.clone(),
            &crate::msg::QueryMsg::VaultConnectionState {
                channel_id: channel_id.to_string(),
                vault: target_vault
            }
        ).unwrap().state;

        assert_eq!(
            queried_connection_state,
            true                        // ! Connection is set
        )
    }


    #[test]
    fn test_unset_connection() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Deploy vault
        let vault = deploy_mock_vault(&mut env);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");

        // Set the connection
        env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            vec![],
            vec![]
        ).unwrap();



        // Tested action: unset the connection
        let response = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: false
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-connection");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", channel_id.to_string())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("vault", target_vault.to_base64())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("state", false.to_string())
        );

        // Verify the connection is not set
        let queried_connection_state: bool = env.get_app().wrap().query_wasm_smart::<VaultConnectionStateResponse>(
            vault.clone(),
            &crate::msg::QueryMsg::VaultConnectionState {
                channel_id: channel_id.to_string(),
                vault: target_vault
            }
        ).unwrap().state;

        assert_eq!(
            queried_connection_state,
            false                       // ! Connection is not set
        )
    }


    #[test]
    fn test_setup_master_after_setup_finish() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Deploy vault
        let vault = deploy_mock_vault(&mut env);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");

        // Finish vault setup
        mock_finish_vault_setup(env.get_app(), vault.clone());


        // Tested action: set connection
        let response_result = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            vec![],
            vec![]
        );


        // Make sure SetConnection fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));
    }


    #[test]
    fn test_set_connection_factory_owner() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Deploy vault
        let vault = deploy_mock_vault(&mut env);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");

        // Finish vault setup
        mock_finish_vault_setup(env.get_app(), vault.clone());


        // Tested action: set connection invoked by factory owner
        let _response = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),     // ! Invoked by the factory owner
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            vec![],
            vec![]
        ).unwrap();


        // Verify the connection is set
        let queried_connection_state: bool = env.get_app().wrap().query_wasm_smart::<VaultConnectionStateResponse>(
            vault.clone(),
            &crate::msg::QueryMsg::VaultConnectionState {
                channel_id: channel_id.to_string(),
                vault: target_vault
            }
        ).unwrap().state;

        assert_eq!(
            queried_connection_state,
            true                        // ! Connection is set
        )
    }


    #[test]
    fn test_set_connection_invalid_caller() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Deploy vault
        let vault = deploy_mock_vault(&mut env);

        let channel_id = "channel_0";
        let target_vault = Binary(b"target_vault".to_vec());


        // Tested action: set connection invoked by factory owner
        let response_result = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("some_other_caller"),     // ! Not setup master nor factory owner
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            vec![],
            vec![]
        );


        // Make sure SetConnection fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));
    }

}