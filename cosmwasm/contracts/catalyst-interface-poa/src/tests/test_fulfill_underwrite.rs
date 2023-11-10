mod test_fulfill_underwrite {

    use catalyst_interface_common::state::{UNDERWRITING_COLLATERAL, UNDERWRITING_COLLATERAL_BASE};
    use catalyst_vault_common::{bindings::Asset, msg::{TotalEscrowedAssetResponse, CommonQueryMsg as VaultQueryMsg, AssetEscrowResponse}};
    use cosmwasm_std::{Uint128, Addr, Binary, Uint64};
    use catalyst_types::{U256, u256};
    use test_helpers::{math::f64_to_uint128, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, UNDERWRITER}, env::CustomTestEnv, asset::CustomTestAsset, contract::{mock_factory_deploy_vault, mock_set_vault_connection}, misc::encode_payload_address};
    use std::str::FromStr;

    use crate::tests::{TestEnv, TestAsset, helpers::{compute_expected_receive_asset, mock_instantiate_interface, vault_contract_storage, encode_mock_send_asset_packet}, parameters::{TEST_VAULT_ASSET_COUNT, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION}};
    use crate::msg::ExecuteMsg as InterfaceExecuteMsg;
    

    pub struct MockTestState {
        pub interface: Addr,
        pub vault: Addr,
        pub from_vault: String,
        pub vault_assets: Vec<TestAsset>,
        pub vault_initial_balances: Vec<Uint128>,
        pub vault_weights: Vec<Uint128>,
        pub to_asset: TestAsset,
        pub to_asset_idx: usize,
        pub u: U256,
        pub min_out: Uint128,
        pub to_account: String,
        pub underwrite_incentive_x16: u16,
        pub calldata: Binary,
        pub underwrite_identifier: String,
        pub expiry: Uint64,
        pub vault_return: Uint128,
        pub interface_escrowed_funds: Uint128
    }
    
    impl MockTestState {
    
        pub fn initialize(
            env: &mut TestEnv
        ) -> Self {
    
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(env.get_app());
            let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
            let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
            let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
            let vault_code_id = vault_contract_storage(env.get_app());
            let vault = mock_factory_deploy_vault::<Asset, _, _>(
                env,
                vault_assets.clone(),
                vault_initial_balances.clone(),
                vault_weights.clone(),
                AMPLIFICATION,
                vault_code_id,
                Some(interface.clone()),
                None,
                None
            );
    
            // Connect vault with a mock vault
            let from_vault = "from_vault".to_string();
            mock_set_vault_connection(
                env.get_app(),
                vault.clone(),
                CHANNEL_ID,
                encode_payload_address(from_vault.as_bytes()),
                true
            );

            // Define the receive asset configuration
            let to_asset_idx = 0;
            let to_asset = vault_assets[to_asset_idx].clone();
            let to_weight = vault_weights[to_asset_idx];
            let to_balance = vault_initial_balances[to_asset_idx];
            
            let swap_units = u256!("500000000000000000");
            let underwrite_incentive_x16 = 1u16 << 13u16; // 12.5%

            // Get the expected swap return
            let expected_return = compute_expected_receive_asset(
                swap_units,
                to_weight,
                to_balance
            );
    
            let underwriter_provided_funds = f64_to_uint128(
                expected_return.to_amount * 1.1
            ).unwrap();
    
            // Fund underwriter with assets
            to_asset.transfer(
                env.get_app(),
                underwriter_provided_funds,
                Addr::unchecked(SETUP_MASTER),
                UNDERWRITER.to_string(),
            );

            // Perform an underwrite
            let response = env.execute_contract(
                Addr::unchecked(UNDERWRITER),
                interface.clone(),
                &InterfaceExecuteMsg::Underwrite {
                    to_vault: vault.to_string(),
                    to_asset_ref: to_asset.get_asset_ref(),
                    u: swap_units,
                    min_out: Uint128::zero(),
                    to_account: SWAPPER_B.to_string(),
                    underwrite_incentive_x16,
                    calldata: Binary::default()
                },
                vec![to_asset.clone()],
                vec![underwriter_provided_funds]
            ).unwrap();

            let underwrite_identifier = response.events[4].attributes[1].value.clone();
            let expiry = Uint64::try_from(
                response.events[4].attributes[3].value.as_str()
            ).unwrap();
            let vault_return = Uint128::from_str(
                &response.events[2].attributes[4].value
            ).unwrap();

            let incentive = vault_return
                * Uint128::from(underwrite_incentive_x16)
                >> 16;
            
            let collateral = vault_return
                * UNDERWRITING_COLLATERAL
                / UNDERWRITING_COLLATERAL_BASE;

            let interface_escrowed_funds = incentive + collateral;
    
            Self {
                interface,
                vault,
                from_vault,
                vault_assets,
                vault_initial_balances,
                vault_weights,
                to_asset,
                to_asset_idx,
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16,
                calldata: Binary::default(),
                underwrite_identifier,
                expiry,
                vault_return,
                interface_escrowed_funds
            }
        }
    }


    #[test]
    fn test_fulfill_underwrite_and_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault,
            vault_assets: _,
            vault_initial_balances: _,
            vault_weights: _,
            to_asset,
            to_asset_idx,
            u,
            min_out,
            to_account: _,
            underwrite_incentive_x16,
            calldata,
            underwrite_identifier,
            expiry: _,
            vault_return,
            interface_escrowed_funds
        } = MockTestState::initialize(&mut env);

        let mock_packet = encode_mock_send_asset_packet(
            from_vault,
            vault.to_string(),
            SWAPPER_B,
            u,
            to_asset_idx as u8,
            min_out.into(),
            U256::zero(),   // Used only for events
            "from_asset",   // Used only for events
            0u32,           // Used only for events
            underwrite_incentive_x16,
            calldata
        );

        let prior_underwriter_balance = to_asset.query_balance(env.get_app(), UNDERWRITER);
        

        // Tested action: fulfill the underwrite
        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            interface.clone(),
            &InterfaceExecuteMsg::PacketReceive {
                data: mock_packet,
                channel_id: CHANNEL_ID
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the underwrite fulfill event
        let event = response.events[2].clone();
        assert_eq!(
            event.ty,
            "wasm-fulfill-underwrite"
        );
        assert_eq!(
            event.attributes[1].value,
            underwrite_identifier
        );

        // Verify funds transfers
        let queried_underwriter_balance = to_asset.query_balance(env.get_app(), UNDERWRITER);

        assert_eq!(
            queried_underwriter_balance - prior_underwriter_balance,
            interface_escrowed_funds + vault_return
        );

        let queried_interface_balance = to_asset.query_balance(env.get_app(), interface);

        assert!(queried_interface_balance.is_zero());

        // Verify vault escrow is released
        let queried_escrowed_total = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                vault.clone(),
                &VaultQueryMsg::TotalEscrowedAsset { asset_ref: to_asset.get_asset_ref() }
            )
            .unwrap()
            .amount;

        assert!(queried_escrowed_total.is_zero());

        let queried_fallback_account = env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                vault.clone(),
                &VaultQueryMsg::AssetEscrow {
                    hash: Binary::from_base64(&underwrite_identifier).unwrap()
                }
            )
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            None
        );

    }

}
