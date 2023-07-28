
mod test_volatile_instantiate {
    use cosmwasm_std::{Uint128, Addr, Uint64, WasmMsg, to_binary, Attribute};
    use cw20_base::state::TokenInfo;
    use cw_multi_test::{App, Executor};
    use catalyst_vault_common::msg::{SetupMasterResponse, ChainInterfaceResponse, OnlyLocalResponse, VaultFeeResponse, GovernanceFeeShareResponse};
    use test_helpers::{definitions::DEPLOYER, contract::mock_instantiate_vault_msg};

    use crate::{msg::QueryMsg, tests::helpers::volatile_vault_contract_storage};


    #[test]
    fn test_instantiate() {

        let mut app = App::default();

        let chain_interface = Some("chain_interface".to_string());
        let instantiate_msg = mock_instantiate_vault_msg(chain_interface);



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

        // Query and verify vault fee
        let vault_fee: Uint64 = app
            .wrap()
            .query_wasm_smart::<VaultFeeResponse>(vault_contract.clone(), &QueryMsg::VaultFee {})
            .unwrap()
            .fee;

        assert_eq!(
            vault_fee,
            instantiate_msg.vault_fee
        );

        // Query and verify governance fee
        let gov_fee_share: Uint64 = app
            .wrap()
            .query_wasm_smart::<GovernanceFeeShareResponse>(vault_contract.clone(), &QueryMsg::GovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            gov_fee_share,
            instantiate_msg.governance_fee_share
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
                decimals: 6,
                total_supply: Uint128::zero(),
                mint: None
            }
        );

    }


    #[test]
    fn test_instantiate_only_local() {

        let mut app = App::default();

        let chain_interface = None;
        let instantiate_msg = mock_instantiate_vault_msg(chain_interface);



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


    #[test]
    fn test_instantiate_events() {

        let mut app = App::default();

        let chain_interface = Some("chain_interface".to_string());
        let instantiate_msg = mock_instantiate_vault_msg(chain_interface);



        // Tested action: instantiate contract
        let contract_code_storage = volatile_vault_contract_storage(&mut app);

        let wasm_instantiate_msg = WasmMsg::Instantiate {
            admin: None,
            code_id: contract_code_storage,
            msg: to_binary(&instantiate_msg).unwrap(),
            funds: vec![],
            label: "volatile_vault".into(),
        };

        let response = app.execute(
            Addr::unchecked(DEPLOYER),
            wasm_instantiate_msg.into()
        ).unwrap();

    

        // Check the events
        let fee_administrator_event = response.events[1].clone();
        assert_eq!(fee_administrator_event.ty, "wasm-set-fee-administrator");
        assert_eq!(
            fee_administrator_event.attributes[1],
            Attribute::new("administrator", instantiate_msg.fee_administrator.to_string())
        );

        let vault_fee_event = response.events[2].clone();
        assert_eq!(vault_fee_event.ty, "wasm-set-vault-fee");
        assert_eq!(
            vault_fee_event.attributes[1],
            Attribute::new("fee", instantiate_msg.vault_fee.to_string())
        );

        let governance_fee_event = response.events[3].clone();
        assert_eq!(governance_fee_event.ty, "wasm-set-governance-fee-share");
        assert_eq!(
            governance_fee_event.attributes[1],
            Attribute::new("fee", instantiate_msg.governance_fee_share.to_string())
        );

    }

}