mod test_volatile_fees {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Addr, from_binary};
    use swap_pool_common::ContractError;

    use crate::{msg::VolatileExecuteMsg, tests::helpers::{mock_instantiate, FACTORY_OWNER_ADDR, FEE_ADMINISTRATOR}, contract::{execute, query}};



    // Set Fee Administrator Tests **********************************************************************************************

    #[test]
    fn test_set_fee_administrator() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_fee_administrator: &str = "new_fee_administrator";


        // Tested action: set fee administrator
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FACTORY_OWNER_ADDR, &[]),
            VolatileExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() }
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new fee administrator is set
        let queried_fee_administrator: Addr = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::FeeAdministrator {}).unwrap()
        ).unwrap();

        assert_eq!(
            queried_fee_administrator,
            Addr::unchecked(new_fee_administrator.to_string())
        );

    }


    #[test]
    fn test_set_fee_administrator_invalid_caller() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_fee_administrator: &str = "new_fee_administrator";


        // Tested action: set fee administrator
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("not_factory_owner", &[]),     // ! Not FACTORY_OWNER_ADDR
            VolatileExecuteMsg::SetFeeAdministrator { administrator: new_fee_administrator.to_string() }
        );


        // Make sure SetFeeAdministrator fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }



    // Set Pool Fee Tests *******************************************************************************************************
    
    #[test]
    fn test_set_pool_fee() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_pool_fee: u64 = 500;


        // Tested action: set pool fee
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FEE_ADMINISTRATOR, &[]),
            VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee }
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new pool fee is set
        let queried_pool_fee: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::PoolFee {}).unwrap()
        ).unwrap();

        assert_eq!(
            queried_pool_fee,
            new_pool_fee
        );

    }


    #[test]
    fn test_set_pool_fee_max() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_pool_fee: u64 = 1000000000000000000u64;


        // Tested action: set max pool fee
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FEE_ADMINISTRATOR, &[]),
            VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee }
        ).unwrap();


        // Verify the max pool fee is set
        let queried_pool_fee: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::PoolFee {}).unwrap()
        ).unwrap();

        assert_eq!(
            queried_pool_fee,
            new_pool_fee
        );

    }


    #[test]
    fn test_set_pool_fee_larger_than_max() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_pool_fee: u64 = 1000000000000000000u64 + 1u64;


        // Tested action: set pool fee larger than maximum allowed
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FEE_ADMINISTRATOR, &[]),
            VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee }
        );


        // Make sure SetPoolFee fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::InvalidPoolFee { requested_fee, max_fee }
                if requested_fee == new_pool_fee && max_fee == 1000000000000000000u64
        ));

    }


    #[test]
    fn test_set_pool_fee_invalid_caller() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_pool_fee: u64 = 1000000000000000000u64 + 1u64;


        // Tested action: set pool fee with invalid caller
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("not_fee_administrator", &[]),        // ! Not FEE_ADMINISTRATOR
            VolatileExecuteMsg::SetPoolFee { fee: new_pool_fee }
        );


        // Make sure SetPoolFee fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }



    // Set Governance Fee Share Tests *******************************************************************************************
    
    #[test]
    fn test_set_gov_fee_share() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_gov_fee_share: u64 = 500;


        // Tested action: set governance fee share
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FEE_ADMINISTRATOR, &[]),
            VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share }
        ).unwrap();

        
        // TODO verify response attributes (event)

        // Verify the new governance fee share is set
        let queried_gov_fee_share: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::GovernanceFeeShare {}).unwrap()
        ).unwrap();

        assert_eq!(
            queried_gov_fee_share,
            new_gov_fee_share
        );

    }


    #[test]
    fn test_set_gov_fee_share_max() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_gov_fee_share: u64 = 750000000000000000u64;


        // Tested action: set max governance fee share
        let _response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FEE_ADMINISTRATOR, &[]),
            VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share }
        ).unwrap();


        // Verify the max governance fee share is set
        let queried_gov_fee_share: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), crate::msg::QueryMsg::GovernanceFeeShare {}).unwrap()
        ).unwrap();

        assert_eq!(
            queried_gov_fee_share,
            new_gov_fee_share
        );

    }


    #[test]
    fn test_set_gov_fee_share_larger_than_max() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_gov_fee_share: u64 = 750000000000000000u64 + 1u64;


        // Tested action: set governance fee share larger than maximum allowed
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(FEE_ADMINISTRATOR, &[]),
            VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share }
        );


        // Make sure SetGovernanceFeeShare fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::InvalidGovernanceFee { requested_fee, max_fee }
                if requested_fee == new_gov_fee_share && max_fee == 750000000000000000u64
        ));

    }


    #[test]
    fn test_set_gov_fee_share_invalid_caller() {
    
        let mut deps = mock_dependencies();
        mock_instantiate(deps.as_mut(), false);

        let new_gov_fee_share: u64 = 1000000000000000000u64 + 1u64;


        // Tested action: set governance fee share with invalid caller
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("not_fee_administrator", &[]),        // ! Not FEE_ADMINISTRATOR
            VolatileExecuteMsg::SetGovernanceFeeShare { fee: new_gov_fee_share }
        );


        // Make sure SetGovernanceFeeShare fails
        assert!(matches!(
            response_result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


}