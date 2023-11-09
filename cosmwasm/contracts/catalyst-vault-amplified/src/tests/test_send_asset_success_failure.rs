mod test_amplified_send_asset_success_failure {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedAssetResponse, AssetEscrowResponse}, state::compute_send_asset_hash, bindings::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128}, misc::{encode_payload_address, get_response_attribute}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_B, SWAPPER_A, FACTORY_OWNER}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_set_vault_connection}, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::{TestEnv, TestAsset};
    use crate::{msg::{AmplifiedExecuteMsg, QueryMsg}, tests::{helpers::amplified_vault_contract_storage, parameters::{AMPLIFICATION, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT}}};



    struct MockTest {
        pub interface: Addr,
        pub vault: Addr,
        pub vault_assets: Vec<TestAsset>,
        pub vault_initial_balances: Vec<Uint128>,
        pub from_asset_idx: usize,
        pub from_asset: TestAsset,
        pub from_amount: Uint128,
        pub fee: Uint128,
        pub u: U256,
        pub to_account: Binary,
        pub block_number: u32
    }

    impl MockTest {

        pub fn initiate_mock(test_env: &mut TestEnv) -> Self {
            // Instantiate and initialize vault
            let interface = mock_instantiate_interface(test_env.get_app());
            let vault_assets = test_env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
            let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
            let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
            let vault_code_id = amplified_vault_contract_storage(test_env.get_app());
            let vault = mock_factory_deploy_vault::<Asset, _, _>(
                test_env,
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
            let target_vault = encode_payload_address(b"target_vault");
            mock_set_vault_connection(
                test_env.get_app(),
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
    
            // Fund swapper with tokens
            from_asset.transfer(
                test_env.get_app(),
                swap_amount,
                Addr::unchecked(SETUP_MASTER),
                SWAPPER_A.to_string()
            );
    
            // Execute send asset
            let response = test_env.execute_contract(
                Addr::unchecked(SWAPPER_A),
                vault.clone(),
                &AmplifiedExecuteMsg::SendAsset {
                    channel_id: CHANNEL_ID.to_string(),
                    to_vault: target_vault,
                    to_account: to_account.clone(),
                    from_asset_ref: from_asset.get_asset_ref(),
                    to_asset_index: to_asset_idx,
                    amount: swap_amount,
                    min_out: U256::zero(),
                    fallback_account: SWAPPER_A.to_string(),
                    underwrite_incentive_x16: 0u16,
                    calldata: Binary(vec![])
                },
                vec![from_asset.clone()],
                vec![swap_amount]
            ).unwrap();

            let u = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
            let fee = get_response_attribute::<Uint128>(response.events[1].clone(), "fee").unwrap();
            let block_number = test_env.get_app().block_info().height as u32;

            MockTest {
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action: send asset ack
        test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_asset_hash(
            mock.to_account.as_slice(),
            mock.u,
            mock.from_amount - mock.fee,
            mock.from_asset.get_asset_ref().as_str(),
            mock.block_number
        );

        let queried_escrow = test_env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                mock.vault.clone(),
                &QueryMsg::AssetEscrow { hash: Binary(swap_hash) }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());


        // Verify total escrowed balance is updated
        let queried_total_escrowed_balances = test_env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                mock.vault.clone(),
                &QueryMsg::TotalEscrowedAsset { asset_ref: mock.from_asset.get_asset_ref() }
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balances.amount,
            Uint128::zero()
        );

        // Verify the swap assets have NOT been returned to the swapper
        let vault_from_asset_balance = mock.from_asset.query_balance(test_env.get_app(), mock.vault.to_string());
        let factory_owner_from_asset_balance = mock.from_asset.query_balance(test_env.get_app(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance,
            mock.vault_initial_balances[mock.from_asset_idx]                // Initial vault supply
                + mock.from_amount                                               // plus swap amount
                - factory_owner_from_asset_balance                              // minus the governance fee
        );

        // Verify the swap assets have NOT been received by the swapper
        let swapper_from_asset_balance = mock.from_asset.query_balance(test_env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_from_asset_balance,
            Uint128::zero()
        );

    }


    #[test]
    fn test_send_asset_failure() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action: send asset timeout
        test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify escrow is released
        let swap_hash = compute_send_asset_hash(
            mock.to_account.as_slice(),
            mock.u,
            mock.from_amount - mock.fee,
            mock.from_asset.get_asset_ref().as_str(),
            mock.block_number
        );

        let queried_escrow = test_env.get_app()
            .wrap()
            .query_wasm_smart::<AssetEscrowResponse>(
                mock.vault.clone(),
                &QueryMsg::AssetEscrow { hash: Binary(swap_hash) }
            ).unwrap();

        assert!(queried_escrow.fallback_account.is_none());

        // Verify total escrowed balance is updated
        let queried_total_escrowed_balances = test_env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedAssetResponse>(
                mock.vault.clone(),
                &QueryMsg::TotalEscrowedAsset { asset_ref: mock.from_asset.get_asset_ref() }
            ).unwrap();
        
        assert_eq!(
            queried_total_escrowed_balances.amount,
            Uint128::zero()
        );

        // Verify the swap assets have been returned to the swapper
        let vault_from_asset_balance = mock.from_asset.query_balance(test_env.get_app(), mock.vault.to_string());
        let factory_owner_from_asset_balance = mock.from_asset.query_balance(test_env.get_app(), FACTORY_OWNER.to_string());
        assert_eq!(
            vault_from_asset_balance,
            mock.vault_initial_balances[mock.from_asset_idx]        // The vault balance returns to the initial vault balance
                + mock.fee                                               // plus the vault fee
                - factory_owner_from_asset_balance                      // except for the governance fee
        );

        // Verify the swap assets have been received by the swapper
        let swapper_to_asset_balance = mock.from_asset.query_balance(test_env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_to_asset_balance,
            mock.from_amount - mock.fee
        );

    }


    #[test]
    fn test_send_asset_success_event() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action: send asset success
        let response = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
            Attribute::new("to_account", mock.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", mock.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", (mock.from_amount - mock.fee).to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("asset_ref", mock.from_asset.get_asset_ref())
        );
        assert_eq!(
            event.attributes[6],
            Attribute::new("block_number_mod", mock.block_number.to_string())
        );

    }


    #[test]
    fn test_send_asset_failure_event() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action: send asset failure
        let response = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
            Attribute::new("to_account", mock.to_account.to_string())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("units", mock.u.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("escrow_amount", (mock.from_amount - mock.fee).to_string())
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("asset_ref", mock.from_asset.get_asset_ref())
        );
        assert_eq!(
            event.attributes[6],
            Attribute::new("block_number_mod", mock.block_number.to_string())
        );

    }


    #[test]
    fn test_send_asset_no_failure_after_success() {

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

        // Execute send asset ack
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();

    

        // Tested action: send asset timeout
        let response_result = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

        // Execute send asset timeout
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();

    

        // Tested action: send asset ack
        let response_result = test_env.execute_contract(
            mock.interface,
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action: send asset ack
        let response_result = test_env.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action: send asset timeout
        let response_result = test_env.execute_contract(
            Addr::unchecked("not_interface"),           // ! Not the interface contract
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
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
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action 1: send asset ack with invalid account
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send asset ack with invalid units
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u + u256!("1"),                              // ! Increased units
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send asset ack with invalid from amount
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: (mock.from_amount - mock.fee) + Uint128::from(1u64),      // ! Increased from amount
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send asset ack with invalid from asset
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: "not_from_asset".to_string(),                // ! Not the original asset
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 5: send asset ack with invalid block number
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: 101u32                            // ! Not the original block number
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account,
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();
    }

    #[test]
    fn test_send_asset_failure_invalid_params() {
        

        // Setup test
        let mut test_env = TestEnv::initialize(SETUP_MASTER.to_string());
        let mock = MockTest::initiate_mock(&mut test_env);

    

        // Tested action 1: send asset ack with invalid account
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: Binary("not_to_account".as_bytes().to_vec()),   // ! Not the chain interface
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 2: send asset ack with invalid units
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u + u256!("1"),                              // ! Increased units
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 3: send asset ack with invalid from amount
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: (mock.from_amount - mock.fee) * Uint128::from(2u64),      // ! Increased from amount
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 4: send asset ack with invalid from asset
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.vault_assets[mock.from_asset_idx+1].get_asset_ref(),   // ! Not the original asset
                block_number_mod: mock.block_number 
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails

    

        // Tested action 5: send asset ack with invalid block number
        let response_result = test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account.clone(),
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: 101u32                            // ! Not the original block number
            },
            vec![],
            vec![]
        );
        assert!(response_result.is_err());                          // Make sure the transaction fails



        // Make sure the ack works with valid parameters
        test_env.execute_contract(
            mock.interface.clone(),
            mock.vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock.to_account,
                u: mock.u,
                escrow_amount: mock.from_amount - mock.fee,
                asset_ref: mock.from_asset.get_asset_ref(),
                block_number_mod: mock.block_number,
            },
            vec![],
            vec![]
        ).unwrap();
        
    }

}