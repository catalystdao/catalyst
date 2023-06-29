mod test_amplified_fees {
    use cosmwasm_std::{Addr, Uint128, Uint64};
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::{ContractError, msg::{FeeAdministratorResponse, VaultFeeResponse, GovernanceFeeShareResponse}};
    use fixed_point_math::WAD;
    use test_helpers::{token::deploy_test_tokens, definitions::{SETUP_MASTER, FACTORY_OWNER}, contract::mock_factory_deploy_vault};

    use crate::{msg::{AmplifiedExecuteMsg, QueryMsg}, tests::helpers::amplified_vault_contract_storage};



    // Set Fee Administrator Tests **********************************************************************************************
    fn deploy_mock_vault(app: &mut App) -> Addr {
        let vault_tokens = deploy_test_tokens(app, SETUP_MASTER.to_string(), None, None);
        let vault_code_id = amplified_vault_contract_storage(app);
        mock_factory_deploy_vault(
            app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vec![Uint128::from(1u64) * WAD.as_uint128(), Uint128::from(2u64) * WAD.as_uint128(), Uint128::from(3u64) * WAD.as_uint128()],
            vec![Uint64::one(), Uint64::one(), Uint64::one()],
            Uint64::new(900000000000000000u64),
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
        let _response = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() },
            &[]
        ).unwrap();

        
        // TODO verify response attributes (event)

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
        let response_result = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked("not_factory_owner"),   // ! Not FACTORY_OWNER
            vault.clone(),
            &AmplifiedExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() },
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
        let _response = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetVaultFee { fee: new_vault_fee },
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
        let _response = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetVaultFee { fee: new_vault_fee },
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
        let response_result = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetVaultFee { fee: new_vault_fee },
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
        let response_result = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked("not_fee_administrator"),       // ! Not FACTORY_OWNER
            vault.clone(),
            &AmplifiedExecuteMsg::SetVaultFee { fee: new_vault_fee },
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
        let _response = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
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
        let _response = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
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
        let response_result = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
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
        let response_result = app.execute_contract::<AmplifiedExecuteMsg>(
            Addr::unchecked("not_fee_administrator"),       // ! Not FACTORY_OWNER
            vault.clone(),
            &AmplifiedExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        );


        // Make sure SetGovernanceFeeShare fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


}