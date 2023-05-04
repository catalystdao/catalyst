mod test_volatile_fees {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, Executor};
    use swap_pool_common::{ContractError, msg::{FeeAdministratorResponse, PoolFeeResponse, GovernanceFeeShareResponse}};

    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::helpers::{mock_instantiate_vault, FACTORY_OWNER}};



    // Set Fee Administrator Tests **********************************************************************************************

    #[test]
    fn test_set_fee_administrator() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_fee_administrator: &str = "new_fee_administrator";


        // Tested action: set fee administrator
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() },
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
    #[ignore]
    fn test_set_fee_administrator_invalid_caller() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

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



    // Set Pool Fee Tests *******************************************************************************************************
    
    #[test]
    fn test_set_pool_fee() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_pool_fee: u64 = 500;


        // Tested action: set pool fee
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee },
            &[]
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new pool fee is set
        let queried_pool_fee: u64 = app
            .wrap()
            .query_wasm_smart::<PoolFeeResponse>(vault, &QueryMsg::PoolFee {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_pool_fee,
            new_pool_fee
        );

    }


    #[test]
    fn test_set_pool_fee_max() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_pool_fee: u64 = 1000000000000000000u64;


        // Tested action: set max pool fee
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee },
            &[]
        ).unwrap();


        // Verify the max pool fee is set
        let queried_pool_fee: u64 = app
            .wrap()
            .query_wasm_smart::<PoolFeeResponse>(vault, &QueryMsg::PoolFee {})
            .unwrap()
            .fee;

        assert_eq!(
            queried_pool_fee,
            new_pool_fee
        );

    }


    #[test]
    fn test_set_pool_fee_larger_than_max() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_pool_fee: u64 = 1000000000000000000u64 + 1u64;


        // Tested action: set pool fee larger than maximum allowed
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee },
            &[]
        );


        // Make sure SetPoolFee fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidPoolFee { requested_fee, max_fee }
                if requested_fee == new_pool_fee && max_fee == 1000000000000000000u64
        ));

    }


    #[test]
    fn test_set_pool_fee_invalid_caller() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_pool_fee: u64 = 500;


        // Tested action: set pool fee
        let response_result = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked("not_fee_administrator"),       // ! Not FACTORY_OWNER
            vault.clone(),
            &VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee },
            &[]
        );


        // Make sure SetPoolFee fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }



    // Set Governance Fee Share Tests *******************************************************************************************
    
    #[test]
    fn test_set_gov_fee_share() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_gov_fee_share: u64 = 700;


        // Tested action: set governance fee share
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new governance fee share is set
        let queried_gov_fee_share: u64 = app
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

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_gov_fee_share: u64 = 750000000000000000u64;


        // Tested action: set max governance fee share
        let _response = app.execute_contract::<VolatileExecuteMsg>(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share },
            &[]
        ).unwrap();


        // Verify the max governance fee share is set
        let queried_gov_fee_share: u64 = app
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

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_gov_fee_share: u64 = 750000000000000000u64 + 1u64;


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
                if requested_fee == new_gov_fee_share && max_fee == 750000000000000000u64
        ));

    }


    #[test]
    fn test_set_gov_fee_share_invalid_caller() {

        let mut app = App::default();

        // Instantiate vault
        let vault = mock_instantiate_vault(&mut app, None);

        let new_gov_fee_share: u64 = 700;


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