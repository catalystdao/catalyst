mod test_volatile_vault_connections {
    use cosmwasm_std::{Addr, Uint128, Binary, Uint64};
    use cw_multi_test::{Executor, App};
    use catalyst_vault_common::{ContractError, msg::VaultConnectionStateResponse};
    use fixed_point_math::WAD;
    use test_helpers::{misc::encode_payload_address, token::deploy_test_tokens, definitions::{SETUP_MASTER, FACTORY_OWNER}};

    use crate::{msg::VolatileExecuteMsg, tests::helpers::{mock_finish_vault_setup, mock_factory_deploy_vault, mock_instantiate_interface}};

    fn deploy_mock_vault(app: &mut App) -> Addr {
        let interface = mock_instantiate_interface(app);
        let vault_tokens = deploy_test_tokens(app, SETUP_MASTER.to_string(), None, None);
        mock_factory_deploy_vault(
            app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()],
            vec![Uint64::one(), Uint64::one(), Uint64::one()],
            None,
            Some(interface.clone()),
            None
        )
    }

    #[test]
    fn test_set_connection() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");



        // Tested action: set connection
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            &[]
        ).unwrap();



        // TODO verify response attributes (event)

        // Verify the connection is set
        let queried_connection_state: bool = app.wrap().query_wasm_smart::<VaultConnectionStateResponse>(
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

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");

        // Set the connection
        app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            &[]
        ).unwrap();



        // Tested action: unset the connection
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: false
            },
            &[]
        ).unwrap();



        // Verify the connection is not set
        let queried_connection_state: bool = app.wrap().query_wasm_smart::<VaultConnectionStateResponse>(
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

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");

        // Finish vault setup
        mock_finish_vault_setup(&mut app, vault.clone());


        // Tested action: set connection
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            &[]
        );


        // Make sure SetConnection fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));
    }


    #[test]
    fn test_set_connection_factory_owner() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let channel_id = "channel_0";
        let target_vault = encode_payload_address(b"target_vault");

        // Finish vault setup
        mock_finish_vault_setup(&mut app, vault.clone());


        // Tested action: set connection invoked by factory owner
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),     // ! Invoked by the factory owner
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            &[]
        ).unwrap();


        // Verify the connection is set
        let queried_connection_state: bool = app.wrap().query_wasm_smart::<VaultConnectionStateResponse>(
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

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let channel_id = "channel_0";
        let target_vault = Binary(b"target_vault".to_vec());


        // Tested action: set connection invoked by factory owner
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("some_other_caller"),     // ! Not setup master nor factory owner
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_vault: target_vault.clone(),
                state: true
            },
            &[]
        );


        // Make sure SetConnection fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));
    }

}