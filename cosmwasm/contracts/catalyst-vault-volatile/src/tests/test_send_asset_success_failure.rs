mod test_volatile_send_asset_success_failure {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use cw_multi_test::{App, Executor};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedAssetResponse, AssetEscrowResponse}, state::compute_send_asset_hash};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{encode_payload_address, get_response_attribute}, token::{deploy_test_tokens, transfer_tokens, set_token_allowance, query_token_balance}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, SWAPPER_A, FACTORY_OWNER}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_set_vault_connection}};

    use crate::{msg::{VolatileExecuteMsg, QueryMsg}, tests::{helpers::volatile_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    struct TestEnv {
        pub interface: Addr,
        pub vault: Addr,
        pub vault_assets: Vec<Addr>,
        pub vault_initial_balances: Vec<Uint128>,
        pub from_asset_idx: usize,
        pub from_asset: Addr,
        pub from_amount: Uint128,
        pub fee: Uint128,
        pub u: U256,
        pub to_account: Binary,
        pub block_number: u32
    }

    impl TestEnv {

        pub fn initiate_mock_env(app: &mut App) -> Self {
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(app);
            let vault_assets = deploy_test_tokens(app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
            let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
            let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
            let vault_code_id = volatile_vault_contract_storage(app);
            let vault = mock_factory_deploy_vault(
                app,
                vault_assets.iter().map(|token_addr| token_addr.to_string()).collect(),
                vault_initial_balances.clone(),
                vault_weights.clone(),
                AMPLIFICATION,
                vault_code_id,
                Some(interface.clone()),
                None
            );
    
            // Connect vault with a mock vault
            let target_vault = encode_payload_address(b"target_vault");
            mock_set_vault_connection(
                app,
                vault.clone(),
                CHANNEL_ID.to_string(),
                target_vault.clone(),
                true
            );
    
            // Define send asset configuration
            let from_asset_idx = 0;
            let from_asset = vault_assets[from_asset_idx].clone();
            let from_balance = vault_initial_balances[from_asset_idx];
            let send_percentage = 0.15;
            let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();
    
            let to_asset_idx = 1;
            let to_account = encode_payload_address(SWAPPER_B.as_bytes());
    
            // Fund swapper with tokens and set vault allowance
            transfer_tokens(
                app,
                swap_amount,
                from_asset.clone(),
                Addr::unchecked(SETUP_MASTER),
                SWAPPER_A.to_string()
            );
    
            set_token_allowance(
                app,
                swap_amount,
                from_asset.clone(),
                Addr::unchecked(SWAPPER_A),
                vault.to_string()
            );
    
            // Execute send asset
            let response = app.execute_contract(
                Addr::unchecked(SWAPPER_A),
                vault.clone(),
                &VolatileExecuteMsg::SendAsset {
                    channel_id: CHANNEL_ID.to_string(),
                    to_vault: target_vault,
                    to_account: to_account.clone(),
                    from_asset: from_asset.to_string(),
                    to_asset_index: to_asset_idx,
                    amount: swap_amount,
                    min_out: U256::zero(),
                    fallback_account: SWAPPER_A.to_string(),
                    calldata: Binary(vec![])
                },
                &[]
            ).unwrap();

            let u = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
            let fee = get_response_attribute::<Uint128>(response.events[1].clone(), "fee").unwrap();
            let block_number = app.block_info().height as u32;

            TestEnv {
                interface,
                vault,
                vault_assets,
                vault_initial_balances,
                from_asset_idx,
                from_asset,
                from_amount: swap_amount,
                fee,
                u,
                to_account,
                block_number
            }
    
        }

    }


    #[test]
    fn test_send_asset_success() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send asset ack
        app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_asset_hash(
            env.to_account.as_slice(),
            env.u,
            env.from_amount - env.fee,
            env.from_asset.as_ref(),
            env.block_number
        );

        let queried_escrow = app
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                env.vault.clone(),
                &QueryMsg::AssetEscrow { hash: Binary(swap_hash) }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());


        // Verify total escrowed balance is updated
        let queried_total_escrowed_balances = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                env.vault.clone(),
                &QueryMsg::TotalEscrowedAsset { asset: env.from_asset.to_string() }
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balances.amount,
            Uint128::zero()
        );

        // Verify the swap assets have NOT been returned to the swapper
        let vault_from_asset_balance = query_token_balance(&mut app, env.from_asset.clone(), env.vault.to_string());
        let factory_owner_from_asset_balance = query_token_balance(&mut app, env.from_asset.clone(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance,
            env.vault_initial_balances[env.from_asset_idx]                // Initial vault supply
                + env.from_amount                                               // plus swap amount
                - factory_owner_from_asset_balance                              // minus the governance fee
        );

        // Verify the swap assets have NOT been received by the swapper
        let swapper_from_asset_balance = query_token_balance(&mut app, env.from_asset.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_send_asset_failure() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send asset timeout
        app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_asset_hash(
            env.to_account.as_slice(),
            env.u,
            env.from_amount - env.fee,
            env.from_asset.as_ref(),
            env.block_number
        );

        let queried_escrow = app
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                env.vault.clone(),
                &QueryMsg::AssetEscrow { hash: Binary(swap_hash) }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());

        // Verify total escrowed balance is updated
        let queried_total_escrowed_balances = app
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                env.vault.clone(),
                &QueryMsg::TotalEscrowedAsset { asset: env.from_asset.to_string() }
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balances.amount,
            Uint128::zero()
        );

        // Verify the swap assets have been returned to the swapper
        let vault_from_asset_balance = query_token_balance(&mut app, env.from_asset.clone(), env.vault.to_string());
        let factory_owner_from_asset_balance = query_token_balance(&mut app, env.from_asset.clone(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance,
            env.vault_initial_balances[env.from_asset_idx]        // The vault balance returns to the initial vault balance
                + env.fee                                               // plus the vault fee
                - factory_owner_from_asset_balance                      // except for the governance fee
        );

        // Verify the swap assets have been received by the swapper
        let swapper_to_asset_balance = query_token_balance(&mut app, env.from_asset.clone(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            env.from_amount - env.fee
        );

    }


    #[test]
    fn test_send_asset_success_event() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send asset success
        let response = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-asset-success");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_account", env.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", env.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", (env.from_amount - env.fee).to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("asset", env.from_asset.to_string())
        );
        assert_eq!(
            event.attributes[6],
            Attribute::new("block_number_mod", env.block_number.to_string())
        );

    }


    #[test]
    fn test_send_asset_failure_event() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send asset failure
        let response = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-asset-failure");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID)
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_account", env.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", env.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", (env.from_amount - env.fee).to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("asset", env.from_asset.to_string())
        );
        assert_eq!(
            event.attributes[6],
            Attribute::new("block_number_mod", env.block_number.to_string())
        );

    }


    #[test]
    fn test_send_asset_no_failure_after_success() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

        // Execute send asset ack
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();

    

        // Tested action: send asset timeout
        let response_result = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure transaction fails
        assert!(
            format!(
                "{}", response_result.err().unwrap().root_cause()
            ).contains("Addr not found")
        )

    }
    

    #[test]
    fn test_send_asset_no_success_after_failure() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

        // Execute send asset timeout
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();

    

        // Tested action: send asset ack
        let response_result = app.execute_contract(
            env.interface,
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure transaction fails
        assert!(
            format!(
                "{}", response_result.err().unwrap().root_cause()
            ).contains("Addr not found")
        )

    }


    #[test]
    fn test_send_asset_success_invalid_caller() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send asset ack
        let response_result = app.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))
    }


    #[test]
    fn test_send_asset_failure_invalid_caller() {

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action: send asset timeout
        let response_result = app.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Unauthorized {}
        ))
    }


    #[test]
    fn test_send_asset_success_invalid_params() {
        

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action 1: send asset ack with invalid account
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send asset ack with invalid units
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u + u256!("1"),                              // ! Increased units
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send asset ack with invalid from amount
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: (env.from_amount - env.fee) + Uint128::from(1u64),      // ! Increased from amount
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send asset ack with invalid from asset
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: "not_from_asset".to_string(),                // ! Not the original asset
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 5: send asset ack with invalid block number
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: 101u32                            // ! Not the original block number
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account,
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();
    }

    #[test]
    fn test_send_asset_failure_invalid_params() {
        

        // Setup test
        let mut app = App::default();
        let env = TestEnv::initiate_mock_env(&mut app);

    

        // Tested action 1: send asset ack with invalid account
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send asset ack with invalid units
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u + u256!("1"),                              // ! Increased units
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send asset ack with invalid from amount
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: (env.from_amount - env.fee) * Uint128::from(2u64),      // ! Increased from amount
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send asset ack with invalid from asset
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.vault_assets[env.from_asset_idx+1].to_string(),   // ! Not the original asset
                block_number_mod: env.block_number 
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 5: send asset ack with invalid block number
        let response_result = app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account.clone(),
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: 101u32                            // ! Not the original block number
            },
            &[]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        app.execute_contract(
            env.interface.clone(),
            env.vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: env.to_account,
                u: env.u,
                escrow_amount: env.from_amount - env.fee,
                asset: env.from_asset.to_string(),
                block_number_mod: env.block_number,
            },
            &[]
        ).unwrap();
        
    }

}