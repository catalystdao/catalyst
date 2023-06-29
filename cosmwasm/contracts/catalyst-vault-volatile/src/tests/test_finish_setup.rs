mod test_volatile_finish_setup {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, msg::SetupMasterResponse};
    use test_helpers::definitions::SETUP_MASTER;

    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::helpers::mock_instantiate_vault};


    #[test]
    fn test_finish_setup() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);


        // Tested action: finish setup
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::FinishSetup {},
            &[]
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the setup master is removed
        let setup_master: Option<Addr> = app
            .wrap()
            .query_wasm_smart::<SetupMasterResponse>(vault, &QueryMsg::SetupMaster {})
            .unwrap()
            .setup_master;

        assert!(setup_master.is_none());

    }


    #[test]
    fn test_invalid_caller() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);


        // Tested action: finish setup
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("not_setup_master"),     // ! Not SETUP_MASTER
            vault.clone(),
            &VolatileExecuteMsg::FinishSetup {},
            &[]
        );


        // Make sure finish_setup fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}