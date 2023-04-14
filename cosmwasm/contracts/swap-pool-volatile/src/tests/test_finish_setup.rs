mod test_volatile_finish_setup {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Addr, from_binary};
    use swap_pool_common::ContractError;

    use crate::{msg::VolatileExecuteMsg, tests::helpers::{mock_instantiate, SETUP_MASTER_ADDR}, contract::{execute, query}};


    #[test]
    fn test_finish_setup() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut());


        // Tested action: finish setup
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(SETUP_MASTER_ADDR, &[]),
            VolatileExecuteMsg::FinishSetup {}
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the setup master is removed
        let setup_master: Option<Addr> = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::SetupMaster {}).unwrap()
        ).unwrap();

        assert!(setup_master.is_none());

    }


    #[test]
    fn test_invalid_caller() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut());


        // Tested action: finish setup
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("not_setup_master", &[]),     // ! Not SETUP_MASTER_ADDR
            VolatileExecuteMsg::FinishSetup {}
        );


        // Make sure finish_setup fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}