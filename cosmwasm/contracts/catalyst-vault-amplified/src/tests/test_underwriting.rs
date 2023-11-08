mod test_underwriting {
    use cosmwasm_std::{Uint128, Addr, Binary};
    use catalyst_types::u256;
    use catalyst_vault_common::{ContractError, bindings::Asset, msg::{TotalEscrowedAssetResponse, AssetEscrowResponse}};
    use test_helpers::{math::uint128_to_f64, misc::{encode_payload_address, get_response_attribute}, definitions::{SETUP_MASTER, CHAIN_INTERFACE, CHANNEL_ID, UNDERWRITER}, contract::{mock_factory_deploy_vault, mock_set_vault_connection}, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::TestEnv;
    use crate::{msg::AmplifiedExecuteMsg, tests::{helpers::{compute_expected_receive_asset, amplified_vault_contract_storage}, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};




    // `UnderwriteAsset` tests
    // ********************************************************************************************

    #[test]
    fn test_underwrite_asset() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let to_weight = vault_weights[to_asset_idx];
        let to_balance = vault_initial_balances[to_asset_idx];
        let swap_units = u256!("500000000000000000");



        // Tested action: underwrite asset
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the event
        let underwrite_asset_event = response.events[1].clone();
        assert_eq!(
            underwrite_asset_event.ty,
            "wasm-underwrite-asset"
        );
        assert_eq!(
            underwrite_asset_event.attributes[1].value,   // identifier
            underwrite_id.to_base64()
        );
        assert_eq!(
            underwrite_asset_event.attributes[2].value,   // to asset ref
            to_asset.alias.to_string()
        );
        assert_eq!(
            underwrite_asset_event.attributes[3].value,   // units
            swap_units.to_string()
        );

        
        // Verify the swap return
        let expected_return = compute_expected_receive_asset(
            swap_units,
            to_weight,
            to_balance,
            AMPLIFICATION
        );

        let observed_return = get_response_attribute::<Uint128>(underwrite_asset_event, "to_amount").unwrap();
        
        assert!(uint128_to_f64(observed_return) <= expected_return.to_amount * 1.000001);
        assert!(uint128_to_f64(observed_return) >= expected_return.to_amount * 0.999999);


        // Verify the assets have been escrowed
        let queried_escrowed_total = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                vault.clone(),
                &crate::msg::QueryMsg::TotalEscrowedAsset { asset_ref: to_asset.get_asset_ref() }
            )
            .unwrap()
            .amount;

        assert_eq!(
            queried_escrowed_total,
            observed_return
        );

        let queried_fallback_account = env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                vault.clone(),
                &crate::msg::QueryMsg::AssetEscrow { hash: underwrite_id }
            )
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            Some(Addr::unchecked("underwriter"))
        );

    }


    #[test]
    fn test_underwrite_asset_twice() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        // Underwrite asset once
        env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();



        // Tested action: Underwrite asset twice
        let response_result = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


    #[test]
    fn test_underwrite_asset_only_chain_interface() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        

        // Tested action: underwrite asset call from not the chain interface
        let response_result = env.execute_contract(
            Addr::unchecked("not_chain_interface"),     // ! Not the chain interface
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_owned(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        );



        // Verify the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));

    }




    // `ReleaseUnderwriteAsset` tests
    // ********************************************************************************************

    #[test]
    fn test_release_underwrite_asset() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        // Underwrite asset
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();

        let underwrite_asset_event = response.events[1].clone();
        let observed_return = get_response_attribute::<Uint128>(underwrite_asset_event, "to_amount").unwrap();

        

        // Tested action: release underwrite asset
        env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReleaseUnderwriteAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                escrow_amount: observed_return,
                recipient: UNDERWRITER.to_string()
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the underwriter has received the escrowed assets
        let queried_balance = env.get_app().wrap().query_balance(
            UNDERWRITER,
            to_asset.denom.to_owned()
        ).unwrap().amount;

        assert_eq!(
            queried_balance,
            observed_return
        );
        
        // Verify the escrow has been released
        let queried_escrowed_total = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                vault.clone(),
                &crate::msg::QueryMsg::TotalEscrowedAsset { asset_ref: to_asset.get_asset_ref() }
            )
            .unwrap()
            .amount;

        assert_eq!(
            queried_escrowed_total,
            Uint128::zero()
        );

        let queried_fallback_account = env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                vault.clone(),
                &crate::msg::QueryMsg::AssetEscrow { hash: underwrite_id }
            )
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            None
        );
        
    }


    #[test]
    fn test_release_underwrite_asset_invalid_id() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();

        // ! Don't underwrite asset

        

        // Tested action: release underwrite asset
        let response_result = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReleaseUnderwriteAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                escrow_amount: Uint128::zero(),
                recipient: UNDERWRITER.to_string()
            },
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert_eq!(
            response_result.err().unwrap().root_cause().to_string(),
            "cosmwasm_std::addresses::Addr not found"   // NOTE: This error is shown, as the vault fails to load the escrow fallback address
        )
    }


    #[test]
    fn test_release_underwrite_asset_only_chain_interface() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            from_vault.clone(),
            true
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        // Underwrite asset
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();

        let underwrite_asset_event = response.events[1].clone();
        let observed_return = get_response_attribute::<Uint128>(underwrite_asset_event, "to_amount").unwrap();

        

        // Tested action: release underwrite call from not the chain interface
        let response_result = env.execute_contract(
            Addr::unchecked("not_chain_interface"),     // ! Not the chain interface
            vault.clone(),
            &AmplifiedExecuteMsg::ReleaseUnderwriteAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                escrow_amount: observed_return,
                recipient: UNDERWRITER.to_string()
            },
            vec![],
            vec![]
        );



        // Verify the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));
    }


    #[test]
    fn test_release_underwrite_asset_no_connection() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Connect vault with a mock vault
        let from_vault = encode_payload_address(b"from_vault");
        // ! Do not set the vault connection

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        // Underwrite asset
        env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();

        

        // Tested action: release underwrite asset
        let response_result = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::ReleaseUnderwriteAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: from_vault.clone(),
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                escrow_amount: Uint128::zero(),
                recipient: UNDERWRITER.to_string()
            },
            vec![],
            vec![]
        );



        // Verify the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::VaultNotConnected { channel_id, vault }
                if channel_id == CHANNEL_ID.to_string() && vault == from_vault
        ));
    }




    // `DeleteUnderwriteAsset` tests
    // ********************************************************************************************

    #[test]
    fn test_delete_underwrite_asset() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        // Underwrite asset
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();

        let underwrite_asset_event = response.events[1].clone();
        let observed_return = get_response_attribute::<Uint128>(underwrite_asset_event, "to_amount").unwrap();

        

        // Tested action: delete underwrite asset
        env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::DeleteUnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                escrow_amount: observed_return
            },
            vec![],
            vec![]
        ).unwrap();


        
        // Verify the escrow has been deleted
        let queried_escrowed_total = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                vault.clone(),
                &crate::msg::QueryMsg::TotalEscrowedAsset { asset_ref: to_asset.get_asset_ref() }
            )
            .unwrap()
            .amount;

        assert_eq!(
            queried_escrowed_total,
            Uint128::zero()
        );

        let queried_fallback_account = env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                vault.clone(),
                &crate::msg::QueryMsg::AssetEscrow { hash: underwrite_id }
            )
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            None
        );
        
    }


    #[test]
    fn test_delete_underwrite_asset_only_chain_interface() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)),         // Using a mock address, no need for an interface to be deployed
            None,
            None
        );

        // Define the underwrite asset configuration
        let underwrite_id = Binary::from(vec![1, 2, 3]);
        let to_asset_idx = 0;
        let to_asset = vault_assets[to_asset_idx].clone();
        let swap_units = u256!("500000000000000000");

        // Underwrite asset
        let response = env.execute_contract(
            Addr::unchecked(CHAIN_INTERFACE),
            vault.clone(),
            &AmplifiedExecuteMsg::UnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                min_out: Uint128::zero()
            },
            vec![],
            vec![]
        ).unwrap();

        let underwrite_asset_event = response.events[1].clone();
        let observed_return = get_response_attribute::<Uint128>(underwrite_asset_event, "to_amount").unwrap();

        

        // Tested action: delete underwrite asset
        let response_result = env.execute_contract(
            Addr::unchecked("not_chain_interface"),     // ! Not the chain interface
            vault.clone(),
            &AmplifiedExecuteMsg::DeleteUnderwriteAsset {
                identifier: underwrite_id.clone(),
                asset_ref: to_asset.alias.to_string(),
                u: swap_units,
                escrow_amount: observed_return
            },
            vec![],
            vec![]
        );


        
        // Verify the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ));
        
    }



}