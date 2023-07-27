mod test_volatile_fees {
    use cosmwasm_std::{Addr, Uint64, Attribute};
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, msg::{FeeAdministratorResponse, VaultFeeResponse, GovernanceFeeShareResponse}};
    use test_helpers::{token::deploy_test_tokens, definitions::{SETUP_MASTER, FACTORY_OWNER}, contract::mock_factory_deploy_vault};

    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    // Set Fee Administrator Tests **********************************************************************************************
    fn deploy_mock_vault(app: &mut App) -> Addr {
        let vault_tokens = deploy_test_tokens(app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_code_id = volatile_vault_contract_storage(app);
        mock_factory_deploy_vault(
            app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            TEST_VAULT_BALANCES.to_vec(),
            TEST_VAULT_WEIGHTS.to_vec(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        )
    }

    #[test]
    fn test_set_fee_administrator() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_fee_administrator: &str = "new_fee_administrator";


        // Tested action: set fee administrator
        let response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() },
            &[]
        ).unwrap();

        
        // Verify the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-fee-administrator");

        assert_eq!(
            event.attributes[1],
            Attribute::new("administrator", new_fee_administrator.to_string())
        );

        // Verify the new fee administrator is set
        let queried_fee_administrator: Addr = app
            .wrap()
            .query_wasm_smart::<FeeAdministratorResponse>(vault, &QueryMsg::FeeAdministrator {})
            .unwrap()
            .administrator;

        assert_eq!(
            queried_fee_administrator,
            Addr::unchecked(new_fee_administrator.to_string())
        );

    }


    #[test]
    fn test_set_fee_administrator_invalid_caller() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_fee_administrator: &str = "new_fee_administrator";


        // Tested action: set fee administrator
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("not_factory_owner"),   // ! Not FACTORY_OWNER
            vault.clone(),
            &VolatileExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() },
            &[]
        );


        // Make sure SetFeeAdministrator fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }



    // Set Vault Fee Tests *******************************************************************************************************
    
    #[test]
    fn test_set_vault_fee() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_vault_fee = Uint64::new(500);


        // Tested action: set vault fee
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetVaultFee { fee: new_vault_fee },
            &[]
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new vault fee is set
        let queried_vault_fee: Uint64 = app
            .wrap()
            .query_wasm_smart::<VaultFeeResponse>(vault, &QueryMsg::VaultFee {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_vault_fee,
            new_vault_fee
        );

    }


    #[test]
    fn test_set_vault_fee_max() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_vault_fee = Uint64::new(1000000000000000000u64);


        // Tested action: set max vault fee
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetVaultFee { fee: new_vault_fee },
            &[]
        ).unwrap();


        // Verify the max vault fee is set
        let queried_vault_fee: Uint64 = app
            .wrap()
            .query_wasm_smart::<VaultFeeResponse>(vault, &QueryMsg::VaultFee {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_vault_fee,
            new_vault_fee
        );

    }


    #[test]
    fn test_set_vault_fee_larger_than_max() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_vault_fee = Uint64::new(1000000000000000000u64 + 1u64);


        // Tested action: set vault fee larger than maximum allowed
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetVaultFee { fee: new_vault_fee },
            &[]
        );


        // Make sure SetVaultFee fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidVaultFee { requested_fee, max_fee }
                if requested_fee == new_vault_fee && max_fee == Uint64::new(1000000000000000000u64)
        ));

    }


    #[test]
    fn test_set_vault_fee_invalid_caller() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_vault_fee = Uint64::new(500);


        // Tested action: set vaultt fee
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("not_fee_administrator"),       // ! Not FACTORY_OWNER
            vault.clone(),
            &VolatileExecuteMsg::SetVaultFee { fee: new_vault_fee },
            &[]
        );


        // Make sure SetVaulttFee fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }



    // Set Governance Fee Share Tests *******************************************************************************************
    
    #[test]
    fn test_set_gov_fee_share() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_gov_fee_share = Uint64::new(700);


        // Tested action: set governance fee share
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new governance fee share is set
        let queried_gov_fee_share: Uint64 = app
            .wrap()
            .query_wasm_smart::<GovernanceFeeShareResponse>(vault, &QueryMsg::GovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_gov_fee_share,
            new_gov_fee_share
        );

    }


    #[test]
    fn test_set_gov_fee_share_max() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_gov_fee_share = Uint64::new(750000000000000000u64);


        // Tested action: set max governance fee share
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        ).unwrap();


        // Verify the max governance fee share is set
        let queried_gov_fee_share: Uint64 = app
            .wrap()
            .query_wasm_smart::<GovernanceFeeShareResponse>(vault, &QueryMsg::GovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_gov_fee_share,
            new_gov_fee_share
        );

    }


    #[test]
    fn test_set_gov_fee_share_larger_than_max() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_gov_fee_share = Uint64::new(750000000000000000u64 + 1u64);


        // Tested action: set governance fee share larger than maximum allowed
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        );


        // Make sure SetGovernanceFeeShare fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidGovernanceFee { requested_fee, max_fee }
                if requested_fee == new_gov_fee_share && max_fee == Uint64::new(750000000000000000u64)
        ));

    }


    #[test]
    fn test_set_gov_fee_share_invalid_caller() {

        let mut app = App::default();

        // Deploy vault
        let vault = deploy_mock_vault(&mut app);

        let new_gov_fee_share = Uint64::new(700);


        // Tested action: set governance fee share with invalid caller
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("not_fee_administrator"),       // ! Not FACTORY_OWNER
            vault.clone(),
            &VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        );


        // Make sure SetGovernanceFeeShare fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


}