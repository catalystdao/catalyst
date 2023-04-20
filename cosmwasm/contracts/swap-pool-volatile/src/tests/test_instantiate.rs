
mod test_volatile_instantiate {
    use cosmwasm_std::{Uint128, Addr};
    use cw20_base::state::TokenInfo;
    use cw_multi_test::{App, Executor};
    use swap_pool_common::msg::{SetupMasterResponse, ChainInterfaceResponse, OnlyLocalResponse, PoolFeeResponse, GovernanceFeeShareResponse};

    use crate::{msg::QueryMsg, tests::helpers::{mock_instantiate_msg, DEPLOYER, volatile_vault_contract_storage}};


    #[test]
    fn test_instantiate() {

        let mut app = App::default();

        let chain_interface = Some("chain_interface".to_string());
        let instantiate_msg = mock_instantiate_msg(chain_interface);


        // Tested action: instantiate contract
        let contract_code_storage = volatile_vault_contract_storage(&mut app);
        let vault_contract = app.instantiate_contract(
            contract_code_storage,
            Addr::unchecked(DEPLOYER),
            &instantiate_msg,
            &[],
            "volatile_vault",
            None
        ).unwrap();


        //TODO Check response attributes

        // Query and verify setup master
        let setup_master: Option<Addr> = app
            .wrap()
            .query_wasm_smart::<SetupMasterResponse>(vault_contract.clone(), &QueryMsg::SetupMaster {})
            .unwrap()
            .setup_master;

        assert_eq!(
            setup_master.map(|account| account.to_string()),
            Some(instantiate_msg.setup_master)
        );

        // Query and verify chain interface
        let chain_interface: Option<Addr> = app
            .wrap()
            .query_wasm_smart::<ChainInterfaceResponse>(vault_contract.clone(), &QueryMsg::ChainInterface {})
            .unwrap()
            .chain_interface;

        assert_eq!(
            chain_interface.map(|account| account.to_string()),
            instantiate_msg.chain_interface
        );

        // Query and verify OnlyLocal property
        let only_local: bool = app
            .wrap()
            .query_wasm_smart::<OnlyLocalResponse>(vault_contract.clone(), &QueryMsg::OnlyLocal {})
            .unwrap()
            .only_local;

        assert_eq!(
            only_local,
            false
        );

        // Query and verify pool fee
        let pool_fee: u64 = app
            .wrap()
            .query_wasm_smart::<PoolFeeResponse>(vault_contract.clone(), &QueryMsg::PoolFee {})
            .unwrap()
            .fee;

        assert_eq!(
            pool_fee,
            instantiate_msg.pool_fee
        );

        // Query and verify governance fee
        let gov_fee_share: u64 = app
            .wrap()
            .query_wasm_smart::<GovernanceFeeShareResponse>(vault_contract.clone(), &QueryMsg::GovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            gov_fee_share,
            instantiate_msg.governance_fee
        );

        // Query and verify token info
        let token_info: TokenInfo = app
            .wrap()
            .query_wasm_smart::<TokenInfo>(vault_contract.clone(), &QueryMsg::TokenInfo {})
            .unwrap();

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

        let mut app = App::default();

        let chain_interface = None;
        let instantiate_msg = mock_instantiate_msg(chain_interface);


        // Tested action: instantiate contract
        let contract_code_storage = volatile_vault_contract_storage(&mut app);
        let vault_contract = app.instantiate_contract(
            contract_code_storage,
            Addr::unchecked(DEPLOYER),
            &instantiate_msg,
            &[],
            "volatile_vault",
            None
        ).unwrap();


        //TODO Check response attributes

        // Query and verify chain interface
        let chain_interface: Option<Addr> = app
            .wrap()
            .query_wasm_smart::<ChainInterfaceResponse>(vault_contract.clone(), &QueryMsg::ChainInterface {})
            .unwrap()
            .chain_interface;

        assert_eq!(
            chain_interface.map(|account| account.to_string()),
            None
        );

        // Query and verify OnlyLocal property
        let only_local: bool = app
            .wrap()
            .query_wasm_smart::<OnlyLocalResponse>(vault_contract.clone(), &QueryMsg::OnlyLocal {})
            .unwrap()
            .only_local;

        assert_eq!(
            only_local,
            true
        );
    }

}