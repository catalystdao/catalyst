
mod test_volatile_instantiate {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary, Uint128, Addr};
    use cw20_base::state::TokenInfo;

    use crate::{contract::{instantiate, query}, msg::QueryMsg, tests::helpers::{mock_instantiate_msg, DEPLOYER_ADDR}};


    #[test]
    fn test_instantiate() {

        let mut deps = mock_dependencies();
        let chain_interface = Some("chain_interface".to_string());
        let instantiate_msg = mock_instantiate_msg(chain_interface);


        // Tested action: instantiate contract
        let _response = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            instantiate_msg.clone()
        ).unwrap();


        //TODO Check response attributes

        // Query and verify setup master
        let setup_master: Option<Addr> = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::SetupMaster {}).unwrap()
        ).unwrap();

        assert_eq!(
            setup_master.map(|account| account.to_string()),
            Some(instantiate_msg.setup_master)
        );

        // Query and verify chain interface
        let chain_interface: Option<Addr> = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ChainInterface {}).unwrap()
        ).unwrap();

        assert_eq!(
            chain_interface.map(|account| account.to_string()),
            instantiate_msg.chain_interface
        );

        // Query and verify OnlyLocal property
        let only_local: bool = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::OnlyLocal {}).unwrap()
        ).unwrap();

        assert_eq!(
            only_local,
            false
        );

        // Query and verify pool fee
        let pool_fee: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::PoolFee {}).unwrap()
        ).unwrap();

        assert_eq!(
            pool_fee,
            instantiate_msg.pool_fee
        );

        // Query and verify governance fee
        let gov_fee_share: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::GovernanceFeeShare {}).unwrap()
        ).unwrap();

        assert_eq!(
            gov_fee_share,
            instantiate_msg.governance_fee
        );

        // Query and verify token info
        let token_info: TokenInfo = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo {}).unwrap()
        ).unwrap();

        assert_eq!(
            token_info,
            TokenInfo {
                name: instantiate_msg.name,
                symbol: instantiate_msg.symbol,
                decimals: 18,
                total_supply: Uint128::zero(),
                mint: None
            }
        );

    }


    #[test]
    fn test_instantiate_only_local() {

        let mut deps = mock_dependencies();
        let chain_interface = None;
        let instantiate_msg = mock_instantiate_msg(chain_interface);


        // Tested action: instantiate contract
        let _response = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            instantiate_msg.clone()
        ).unwrap();


        //TODO Check response attributes

        // Query and verify chain interface
        let chain_interface: Option<Addr> = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ChainInterface {}).unwrap()
        ).unwrap();

        assert!(
            chain_interface.is_none()
        );

        // Query and verify OnlyLocal property
        let only_local: bool = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::OnlyLocal {}).unwrap()
        ).unwrap();

        assert_eq!(
            only_local,
            true
        );
    }

}