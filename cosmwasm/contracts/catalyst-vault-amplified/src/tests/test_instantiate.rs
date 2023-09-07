mod test_amplified_instantiate {
    use cosmwasm_std::{Uint128, Addr, Uint64, WasmMsg, to_binary, Attribute};
    use cw_multi_test::Executor;
    use catalyst_vault_common::msg::{SetupMasterResponse, ChainInterfaceResponse, OnlyLocalResponse, VaultFeeResponse, GovernanceFeeShareResponse, TotalSupplyResponse};
    use test_helpers::{definitions::{DEPLOYER, SETUP_MASTER, VAULT_TOKEN_DENOM}, contract::mock_instantiate_vault_msg, env::CustomTestEnv, vault_token::CustomTestVaultToken};

    use crate::tests::{TestEnv, TestVaultToken};
    use crate::{msg::QueryMsg, tests::helpers::amplified_vault_contract_storage};


    #[test]
    fn test_instantiate() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let chain_interface = Some("chain_interface".to_string());
        let instantiate_msg = mock_instantiate_vault_msg(chain_interface);



        // Tested action: instantiate contract
        let contract_code_storage = amplified_vault_contract_storage(env.get_app());
        let vault_contract = env.get_app().instantiate_contract(
            contract_code_storage,
            Addr::unchecked(DEPLOYER),
            &instantiate_msg,
            &[],
            "amplified_vault",
            None
        ).unwrap();



        // Query and verify setup master
        let setup_master: Option<Addr> = env.get_app()
            .wrap()
            .query_wasm_smart::<SetupMasterResponse>(vault_contract.clone(), &QueryMsg::SetupMaster {})
            .unwrap()
            .setup_master;

        assert_eq!(
            setup_master.map(|account| account.to_string()),
            Some(instantiate_msg.setup_master)
        );

        // Query and verify chain interface
        let chain_interface: Option<Addr> = env.get_app()
            .wrap()
            .query_wasm_smart::<ChainInterfaceResponse>(vault_contract.clone(), &QueryMsg::ChainInterface {})
            .unwrap()
            .chain_interface;

        assert_eq!(
            chain_interface.map(|account| account.to_string()),
            instantiate_msg.chain_interface
        );

        // Query and verify OnlyLocal property
        let only_local: bool = env.get_app()
            .wrap()
            .query_wasm_smart::<OnlyLocalResponse>(vault_contract.clone(), &QueryMsg::OnlyLocal {})
            .unwrap()
            .only_local;

        assert_eq!(
            only_local,
            false
        );

        // Query and verify vault fee
        let vault_fee: Uint64 = env.get_app()
            .wrap()
            .query_wasm_smart::<VaultFeeResponse>(vault_contract.clone(), &QueryMsg::VaultFee {})
            .unwrap()
            .fee;

        assert_eq!(
            vault_fee,
            instantiate_msg.vault_fee
        );

        // Query and verify governance fee
        let gov_fee_share: Uint64 = env.get_app()
            .wrap()
            .query_wasm_smart::<GovernanceFeeShareResponse>(vault_contract.clone(), &QueryMsg::GovernanceFeeShare {})
            .unwrap()
            .fee;

        assert_eq!(
            gov_fee_share,
            instantiate_msg.governance_fee_share
        );

        // Query and verify token info
        let vault_token_supply: Uint128 = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalSupplyResponse>(vault_contract.clone(), &QueryMsg::TotalSupply {})
            .unwrap()
            .total_supply;

        assert_eq!(
            vault_token_supply,
            Uint128::zero()
        );

        let vault_token = TestVaultToken::load(vault_contract.to_string(), VAULT_TOKEN_DENOM.to_string());
        let vault_token_info = vault_token.query_token_info(env.get_app());

        assert_eq!(
            vault_token_info.name,
            "TestVault"
        );
        assert_eq!(
            vault_token_info.symbol,
            VAULT_TOKEN_DENOM
        );
        assert_eq!(
            vault_token_info.decimals,
            18
        );

    }


    #[test]
    fn test_instantiate_only_local() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let chain_interface = None;
        let instantiate_msg = mock_instantiate_vault_msg(chain_interface);



        // Tested action: instantiate contract
        let contract_code_storage = amplified_vault_contract_storage(env.get_app());
        let vault_contract = env.get_app().instantiate_contract(
            contract_code_storage,
            Addr::unchecked(DEPLOYER),
            &instantiate_msg,
            &[],
            "amplified_vault",
            None
        ).unwrap();



        // Query and verify chain interface
        let chain_interface: Option<Addr> = env.get_app()
            .wrap()
            .query_wasm_smart::<ChainInterfaceResponse>(vault_contract.clone(), &QueryMsg::ChainInterface {})
            .unwrap()
            .chain_interface;

        assert_eq!(
            chain_interface.map(|account| account.to_string()),
            None
        );

        // Query and verify OnlyLocal property
        let only_local: bool = env.get_app()
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

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let chain_interface = Some("chain_interface".to_string());
        let instantiate_msg = mock_instantiate_vault_msg(chain_interface);



        // Tested action: instantiate contract
        let contract_code_storage = amplified_vault_contract_storage(env.get_app());

        let wasm_instantiate_msg = WasmMsg::Instantiate {
            admin: None,
            code_id: contract_code_storage,
            msg: to_binary(&instantiate_msg).unwrap(),
            funds: vec![],
            label: "amplified_vault".into(),
        };

        let response = env.get_app().execute(
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