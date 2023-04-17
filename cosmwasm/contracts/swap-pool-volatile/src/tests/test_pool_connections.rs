mod test_volatile_pool_connections {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary};
    use swap_pool_common::ContractError;

    use crate::{msg::VolatileExecuteMsg, tests::helpers::{mock_instantiate, SETUP_MASTER_ADDR, finish_pool_setup, FACTORY_OWNER_ADDR}, contract::{execute, query}};


    #[test]
    fn test_set_connection() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();


        // Tested action: set connection
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(SETUP_MASTER_ADDR, &[]),
            VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            }
        ).unwrap();


        // TODO verify response attributes (event)

        // Verify the connection is set
        let queried_connection_state: bool = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                crate::msg::QueryMsg::PoolConnectionState {
                    channel_id: channel_id.to_string(),
                    pool: target_pool
                }
            ).unwrap()
        ).unwrap();

        assert_eq!(
            queried_connection_state,
            true                        // ! Connection is set
        )
    }


    #[test]
    fn test_unset_connection() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();
        
        // Set the connection
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(SETUP_MASTER_ADDR, &[]),
            VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            }
        ).unwrap();


        // TODO verify response attributes (event)

        // Tested action: unset the connection
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(SETUP_MASTER_ADDR, &[]),
            VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: false
            }
        ).unwrap();


        // Verify the connection is not set
        let queried_connection_state: bool = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                crate::msg::QueryMsg::PoolConnectionState {
                    channel_id: channel_id.to_string(),
                    pool: target_pool
                }
            ).unwrap()
        ).unwrap();

        assert_eq!(
            queried_connection_state,
            false                       // ! Connection is not set
        )
    }


    #[test]
    fn test_setup_master_after_setup_finish() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();

        // Finish pool setup
        finish_pool_setup(deps.as_mut());


        // Tested action: set connection
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(SETUP_MASTER_ADDR, &[]),
            VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            }
        );


        // Make sure SetConnection fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));
    }


    #[test]
    fn test_set_connection_factory_owner() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();

        // Finish pool setup
        finish_pool_setup(deps.as_mut());


        // Tested action: set connection invoked by factory owner
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FACTORY_OWNER_ADDR, &[]),     // ! Invoked by the factory owner
            VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            }
        ).unwrap();


        // Verify the connection is set
        let queried_connection_state: bool = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                crate::msg::QueryMsg::PoolConnectionState {
                    channel_id: channel_id.to_string(),
                    pool: target_pool
                }
            ).unwrap()
        ).unwrap();

        assert_eq!(
            queried_connection_state,
            true                        // ! Connection is set
        )
    }


    #[test]
    fn test_set_connection_invalid_caller() {

        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let channel_id = "channel_0";
        let target_pool = b"target_pool".to_vec();


        // Tested action: set connection invoked by factory owner
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("some_other_caller", &[]),     // ! Not setup master nor factory owner
            VolatileExecuteMsg::SetConnection {
                channel_id: channel_id.to_string(),
                to_pool: target_pool.clone(),
                state: true
            }
        );


        // Make sure SetConnection fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));
    }

}