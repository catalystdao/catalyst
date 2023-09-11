mod test_volatile_finish_setup {
    use cosmwasm_std::Addr;
    use catalyst_vault_common::{ContractError, msg::SetupMasterResponse};
    use test_helpers::env::CustomTestEnv;
    use test_helpers::{definitions::SETUP_MASTER, contract::mock_instantiate_vault};

    use crate::tests::TestEnv;
    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::helpers::volatile_vault_contract_storage};


    #[test]
    fn test_finish_setup() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);


        // Tested action: finish setup
        let response = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::FinishSetup {},
            vec![],
            vec![]
        ).unwrap();

        
        // Check the event
        let event = response.events[1].clone();
        assert_eq!(event.ty, "wasm-finish-setup");

        // Verify the setup master is removed
        let setup_master: Option<Addr> = env.get_app()
            .wrap()
            .query_wasm_smart::<SetupMasterResponse>(vault, &QueryMsg::SetupMaster {})
            .unwrap()
            .setup_master;

        assert!(setup_master.is_none());

    }


    #[test]
    fn test_invalid_caller() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate vault
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
        let vault = mock_instantiate_vault(env.get_app(), vault_code_id, None);


        // Tested action: finish setup
        let response_result = env.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("not_setup_master"),     // ! Not SETUP_MASTER
            vault.clone(),
            &VolatileExecuteMsg::FinishSetup {},
            vec![],
            vec![]
        );


        // Make sure finish_setup fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}