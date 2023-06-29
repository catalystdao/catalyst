mod test_amplified_finish_setup {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, msg::SetupMasterResponse};
    use test_helpers::{definitions::SETUP_MASTER, contract::mock_instantiate_vault};

    use crate::{msg::{AmplifiedExecuteMsg, QueryMsg}, tests::helpers::amplified_vault_contract_storage};


    #[test]
    fn test_finish_setup() {

        let mut app = App::default();

        // Instantiate vault
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);


        // Tested action: finish setup
        let _response = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::FinishSetup {},
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
        let vault_code_id = amplified_vault_contract_storage(&mut app);
        let vault = mock_instantiate_vault(&mut app, vault_code_id, None);


        // Tested action: finish setup
        let response_result = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked("not_setup_master"),     // ! Not SETUP_MASTER
            vault.clone(),
            &AmplifiedExecuteMsg::FinishSetup {},
            &[]
        );


        // Make sure finish_setup fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}