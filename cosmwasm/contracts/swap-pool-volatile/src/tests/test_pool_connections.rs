mod test_volatile_pool_connections {
    use cosmwasm_std::Addr;
    use cw_multi_test::{Executor, App};
    use swap_pool_common::{ContractError, msg::PoolConnectionStateResponse};

    use crate::{msg::VolatileExecuteMsg, tests::helpers::{mock_instantiate, SETUP_MASTER, mock_finish_pool_setup, FACTORY_OWNER}};


    #[test]
    fn test_set_connection() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate(&mut app, false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();



        // Tested action: set connection
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            },
            &[]
        ).unwrap();



        // TODO verify response attributes (event)

        // Verify the connection is set
        let queried_connection_state: bool = app.wrap().query_wasm_smart::<PoolConnectionStateResponse>(
            vault.clone(),
            &crate::msg::QueryMsg::PoolConnectionState {
                channel_id: channel_id.to_string(),
                pool: target_pool
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

        // Instantiate vault
        let vault = mock_instantiate(&mut app, false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();

        // Set the connection
        app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
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
                to_pool: target_pool.clone(),
                state: false
            },
            &[]
        ).unwrap();



        // Verify the connection is not set
        let queried_connection_state: bool = app.wrap().query_wasm_smart::<PoolConnectionStateResponse>(
            vault.clone(),
            &crate::msg::QueryMsg::PoolConnectionState {
                channel_id: channel_id.to_string(),
                pool: target_pool
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

        // Instantiate vault
        let vault = mock_instantiate(&mut app, false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();

        // Finish pool setup
        mock_finish_pool_setup(&mut app, vault.clone());


        // Tested action: set connection
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
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

        // Instantiate vault
        let vault = mock_instantiate(&mut app, false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();

        // Finish pool setup
        mock_finish_pool_setup(&mut app, vault.clone());


        // Tested action: set connection invoked by factory owner
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),     // ! Invoked by the factory owner
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            },
            &[]
        ).unwrap();


        // Verify the connection is set
        let queried_connection_state: bool = app.wrap().query_wasm_smart::<PoolConnectionStateResponse>(
            vault.clone(),
            &crate::msg::QueryMsg::PoolConnectionState {
                channel_id: channel_id.to_string(),
                pool: target_pool
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

        // Instantiate vault
        let vault = mock_instantiate(&mut app, false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();


        // Tested action: set connection invoked by factory owner
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("some_other_caller"),     // ! Not setup master nor factory owner
            vault.clone(),
            &VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
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