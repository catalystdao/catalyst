mod test_full_swap_underwrite {
    use cosmwasm_std::{Uint128, Addr, Binary};
    use catalyst_types::{U256, u256};
    use test_helpers::{math::f64_to_uint128, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, UNDERWRITER}, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::{TestEnv, helpers::{compute_expected_receive_asset, encode_mock_packet, MockTestState}};
    use crate::msg::ExecuteMsg as InterfaceExecuteMsg;



    #[test]
    fn test_underwrite_swap() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let MockTestState {
            interface,
            vault,
            from_vault,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        } = MockTestState::initialize(&mut env);

        // Define the receive asset configuration
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        
        let swap_units = u256!("500000000000000000");

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


        
        // Tested action 1: underwrite the swap
        let response = env.execute_contract(
            Addr::unchecked(UNDERWRITER),
            interface.clone(),
            &InterfaceExecuteMsg::Underwrite {
                to_vault: vault.to_string(),
                to_asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero(),
                to_account: SWAPPER_B.to_string(),
                underwrite_incentive_x16: 0u16,
                calldata: Binary::default()
            },
            vec![to_asset.clone()],
            vec![underwriter_provided_funds]
        ).unwrap();

        // Check events
        let vault_event = response.events[2].clone();
        assert_eq!(
            vault_event.ty,
            "wasm-underwrite-asset"
        );

        let interface_event = response.events[4].clone();
        assert_eq!(
            interface_event.ty,
            "wasm-underwrite-swap"
        );

        // Make sure funds have been transferred
        let queried_underwriter_balance = env.get_app()
            .wrap()
            .query_balance(
                UNDERWRITER,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;
        assert!(queried_underwriter_balance < underwriter_provided_funds);  // The underwriter's balance won't be 0, as excess funds will have been refunded

        let queried_interface_balance = env.get_app()
            .wrap()
            .query_balance(
                interface.clone(),
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;
        assert!(queried_interface_balance > Uint128::zero()); // Escrowed funds (incentive + collateral)

        let queried_to_account_balance = env.get_app()
            .wrap()
            .query_balance(
                SWAPPER_B,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;
        assert!(queried_to_account_balance > Uint128::zero());  // End user has been paid




        // Tested action 2: fulfill the underwrite
        let mock_packet = encode_mock_packet(
            from_vault,
            vault.to_string(),
            SWAPPER_B,
            swap_units,
            to_asset_idx as u8,
            U256::zero(),
            U256::zero(),
            "from_asset",
            0u32,
            0u16,
            Binary::default()
        );

        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            interface.clone(),
            &InterfaceExecuteMsg::PacketReceive {
                data: mock_packet,
                channel_id: CHANNEL_ID.to_string()
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

        // Verify the funds have been transferred
        let new_queried_underwriter_balance = env.get_app()
            .wrap()
            .query_balance(
                UNDERWRITER,
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;
        assert!(new_queried_underwriter_balance > queried_underwriter_balance);  // The underwriter has received the incentive/collateral

        let new_queried_interface_balance = env.get_app()
            .wrap()
            .query_balance(
                interface.clone(),
                to_asset.denom.to_string()
            )
            .unwrap()
            .amount;
        assert!(new_queried_interface_balance.is_zero()); // No escrowed funds left

    }
}
