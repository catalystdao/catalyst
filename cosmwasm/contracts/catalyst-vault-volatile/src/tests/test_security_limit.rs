mod test_volatile_security_limit {
    use std::f64::consts::LN_2;

    use catalyst_types::U256;
    use catalyst_vault_common::{msg::GetLimitCapacityResponse, ContractError, state::{INITIAL_MINT_AMOUNT, DECAY_RATE}};
    use cosmwasm_std::{Addr, Uint128, Binary};
    use cw_multi_test::{App, Executor};
    use test_helpers::{token::{deploy_test_tokens, set_token_allowance}, contract::{mock_factory_deploy_vault, mock_instantiate_interface, mock_set_vault_connection}, definitions::{SETUP_MASTER, SWAPPER_B, CHANNEL_ID, SWAPPER_C}, math::{uint128_to_f64, f64_to_uint128, u256_to_f64, f64_to_u256}, misc::{encode_payload_address, get_response_attribute}};

    use crate::{tests::{parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT, AMPLIFICATION}, helpers::volatile_vault_contract_storage}, msg::{QueryMsg, VolatileExecuteMsg}};


    pub const REMOTE_VAULT: &str = "remote_vault_addr";



    // Test helpers *******************************************************************************

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MockVaultConfig {
        interface: Addr,
        vault: Addr,
        assets: Vec<Addr>,
        vault_initial_balances: Vec<Uint128>,
        remote_vault: Binary,
        max_limit_capacity: U256,
        current_limit_capacity: U256
    }

    fn set_mock_vault(
        app: &mut App,
        bias_limit_capacity: f64
    ) -> MockVaultConfig {

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(app);
        let vault_tokens = deploy_test_tokens(app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(app);
        let vault = mock_factory_deploy_vault(
            app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(interface.clone()),
            None
        );

        // Connect the vault with a mock vault
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        mock_set_vault_connection(
            app,
            vault.clone(),
            CHANNEL_ID.to_string(),
            remote_vault.clone(),
            true
        );

        let intitial_limit_capacity = query_limit_capacity(app, vault.clone());

        let current_limit_capacity: U256;
        if bias_limit_capacity != 1. {
            // Perform a 'receive asset' call to bias the limit capacity
            let units_received: U256;
            if bias_limit_capacity == 0. {
                // Avoid rounding errors if bias is at 100%
                units_received = intitial_limit_capacity;
            }
            else {
                units_received = f64_to_u256(
                    u256_to_f64(intitial_limit_capacity) * (1. - bias_limit_capacity)
                ).unwrap();
            }

            app.execute_contract(
                interface.clone(),
                vault.clone(),
                &VolatileExecuteMsg::ReceiveAsset {
                    channel_id: CHANNEL_ID.to_string(),
                    from_vault: remote_vault.clone(),
                    to_asset_index: 0u8,
                    to_account: SETUP_MASTER.to_string(),
                    u: units_received,
                    min_out: Uint128::zero(),
                    from_amount: U256::zero(),
                    from_asset: Binary("from_asset".as_bytes().to_vec()),
                    from_block_number_mod: 0u32,
                    calldata_target: None,
                    calldata: None
                },
                &[]
            ).unwrap();

            current_limit_capacity = query_limit_capacity(app, vault.clone());
        }
        else {
            current_limit_capacity = intitial_limit_capacity;
        }

        MockVaultConfig {
            interface,
            vault,
            assets: vault_tokens,
            vault_initial_balances,
            remote_vault,
            max_limit_capacity: intitial_limit_capacity,
            current_limit_capacity
        }

    }


    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MockSendAsset {
        to_account: Binary,
        from_asset: Addr,
        swap_amount: Uint128,
        units: U256,
        fee: Uint128
    }

    fn execute_mock_send_asset(
        app: &mut App,
        mock_vault_config: MockVaultConfig,
        send_percentage: f64
    ) -> MockSendAsset {

        let from_asset_idx = 0;
        let from_asset = mock_vault_config.assets[from_asset_idx].clone();
        let from_balance = mock_vault_config.vault_initial_balances[from_asset_idx];
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Set swap allowance
        set_token_allowance(
            app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            mock_vault_config.vault.to_string()
        );

        // Execute send asset
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        let response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            mock_vault_config.vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                from_asset: from_asset.to_string(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();

        let observed_units = get_response_attribute::<U256>(
            response.events[1].clone(),
            "units"
        ).unwrap();

        let observed_fee = get_response_attribute::<Uint128>(
            response.events[1].clone(),
            "fee"
        ).unwrap();

        MockSendAsset {
            to_account,
            from_asset,
            swap_amount,
            units: observed_units,
            fee: observed_fee
        }

    }


    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MockSendLiquidity {
        to_account: Binary,
        swap_amount: Uint128,
        units: U256
    }

    fn execute_mock_send_liquidity(
        app: &mut App,
        mock_vault_config: MockVaultConfig,
        send_percentage: f64
    ) -> MockSendLiquidity {

        let swap_amount = f64_to_uint128(
            uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage
        ).unwrap();

        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Execute send liquidity
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        let response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            mock_vault_config.vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();

        let observed_units = get_response_attribute::<U256>(
            response.events[1].clone(),
            "units"
        ).unwrap();

        MockSendLiquidity {
            to_account,
            swap_amount,
            units: observed_units
        }

    }

    fn query_limit_capacity(
        app: &mut App,
        vault: Addr
    ) -> U256 {
        app.wrap().query_wasm_smart::<GetLimitCapacityResponse>(
            vault.clone(),
            &QueryMsg::GetLimitCapacity {}
        ).unwrap().capacity
    }



    // Tests **************************************************************************************

    #[test]
    fn test_security_limit_initialization() {

        let mut app = App::default();

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(&mut app);
        let vault_tokens = deploy_test_tokens(&mut app, SETUP_MASTER.to_string(), None, TEST_VAULT_ASSET_COUNT);
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(&mut app);



        // Tested action: intialize a new vault
        let vault = mock_factory_deploy_vault(
            &mut app,
            vault_tokens.iter().map(|token_addr| token_addr.to_string()).collect(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(interface.clone()),
            None
        );



        // Check the limit capacity
        let queried_limit_capacity = u256_to_f64(query_limit_capacity(&mut app, vault.clone()));

        let expected_limit_capacity = uint128_to_f64(vault_weights.iter().sum())
            * LN_2
            * 1e18; // Multiplied by 1e18 as the queried limit capacity is in WAD notation

        assert!(queried_limit_capacity <= expected_limit_capacity * 1.000001);
        assert!(queried_limit_capacity >= expected_limit_capacity * 0.999999);

    }


    #[test]
    fn test_security_limit_decay() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.         // ! Decrease the initial limit capacity by 100%
        );
        let vault = mock_vault_config.vault.clone();

        // Make sure capacity is at zero
        assert!(mock_vault_config.current_limit_capacity.is_zero());

        // Check the capacity calculation at different intervals
        let start_timestamp = app.block_info().time;
        let check_steps = vec![0.2, 0.7, 1., 1.1];

        check_steps.iter().for_each(|step| {

            let time_elapsed = (u256_to_f64(DECAY_RATE) * step) as u64;
            app.update_block(|block| {
                block.time = start_timestamp.plus_seconds(time_elapsed);
            });

            let queried_capacity = u256_to_f64(
                query_limit_capacity(&mut app, vault.clone())
            );

            let expected_capacity = u256_to_f64(
                mock_vault_config.max_limit_capacity
            ) * step.min(1.);

            assert!(queried_capacity <= expected_capacity * 1.0001);
            assert!(queried_capacity >= expected_capacity * 0.9999);
        })

    }


    // Asset swap tests ***************************************************************************

    #[test]
    fn test_security_limit_change_on_send_asset() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.5         // ! Decrease the initial limit capacity by 50%
        );
        let vault = mock_vault_config.vault.clone();

        // Define send asset configuration
        let from_asset_idx = 0;
        let from_asset = mock_vault_config.assets[from_asset_idx].clone();
        let from_balance = mock_vault_config.vault_initial_balances[from_asset_idx];
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Set swap allowance
        set_token_allowance(
            &mut app,
            swap_amount,
            from_asset.clone(),
            Addr::unchecked(SETUP_MASTER),
            vault.to_string()
        );



        // Tested action: send asset
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        let response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                from_asset: from_asset.to_string(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();



        // Make sure some units have been sent (i.e. that the test is correctly setup)
        let observed_units = get_response_attribute::<U256>(
            response.events[1].clone(),
            "units"
        ).unwrap();
        assert!(!observed_units.is_zero());

        // Make sure the security limit has not increased (as it does not get increased until the ACK
        // is received)
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        )
        
    }


    #[test]
    fn test_security_limit_change_on_send_asset_success() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.5         // ! Decrease the initial limit capacity by 50%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action 1: small send asset success
        let mock_send_asset_result = execute_mock_send_asset(
            &mut app,
            mock_vault_config.clone(),
            0.15
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_asset_result.units.is_zero());

        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: mock_send_asset_result.swap_amount - mock_send_asset_result.fee,
                asset: mock_send_asset_result.from_asset.to_string(),
                block_number_mod: app.block_info().height as u32,
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity has increased by the received amount.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity + mock_send_asset_result.units
        );



        // Tested action 2: very large send asset success (saturate the limit capacity)
        let mock_send_asset_result = execute_mock_send_asset(
            &mut app,
            mock_vault_config.clone(),
            1.
        );

        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: mock_send_asset_result.swap_amount - mock_send_asset_result.fee,
                asset: mock_send_asset_result.from_asset.to_string(),
                block_number_mod: app.block_info().height as u32,
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity has increased by the received amount.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.max_limit_capacity    // ! Vault capacity is at the maximum
        );

    }


    #[test]
    fn test_security_limit_change_on_send_asset_timeout() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.5         // ! Decrease the initial limit capacity by 50%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action: send asset failure
        let mock_send_asset_result = execute_mock_send_asset(
            &mut app,
            mock_vault_config.clone(),
            0.15
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_asset_result.units.is_zero());

        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: mock_send_asset_result.swap_amount - mock_send_asset_result.fee,
                asset: mock_send_asset_result.from_asset.to_string(),
                block_number_mod: app.block_info().height as u32,
            },
            &[]
        ).unwrap();



        // Make sure the security limit capacity has not changed.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );
        
    }


    #[test]
    fn test_security_limit_change_on_receive_asset() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.5         // ! Decrease the initial limit capacity by 50%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action 1: small receive asset
        let received_units_percentage = 0.15;
        let units = f64_to_u256(
            u256_to_f64(mock_vault_config.current_limit_capacity) * received_units_percentage
        ).unwrap();
        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_asset_index: 0u8,
                to_account: SETUP_MASTER.to_string(),
                u: units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity has decreased by the received amount.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity - units
        );



        // Tested action 2: too large receive asset
        let units = observed_limit_capacity + U256::one();  // ! One unit more than allowed.
        let response_result = app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_asset_index: 0u8,
                to_account: SETUP_MASTER.to_string(),
                u: units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SecurityLimitExceeded { overflow }
                if overflow == U256::one()
        ));



        // Tested action 3: max receive asset
        let units = observed_limit_capacity;
        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_asset_index: 0u8,
                to_account: SETUP_MASTER.to_string(),
                u: units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity is at zero.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert!(observed_limit_capacity.is_zero());

    }


    // Liquidity swap tests ***********************************************************************

    #[test]
    fn test_security_limit_change_on_send_liquidity() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.5         // ! Decrease the initial limit capacity by 50%
        );
        let vault = mock_vault_config.vault.clone();

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(
            uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage
        ).unwrap();

        let to_account = encode_payload_address(SWAPPER_B.as_bytes());



        // Tested action: send liquidity
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        let response = app.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            &[]
        ).unwrap();



        // Make sure some units have been sent (i.e. that the test is correctly setup)
        let observed_units = get_response_attribute::<U256>(
            response.events[1].clone(),
            "units"
        ).unwrap();
        assert!(!observed_units.is_zero());

        // Make sure the security limit has not increased (as it does not get increased until the ACK
        // is received)
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        )
        
    }


    #[test]
    fn test_security_limit_change_on_send_liquidity_success() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.8         // ! Decrease the initial limit capacity by 20%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action 1: small send liquidity success
        let mock_send_liquidity_result = execute_mock_send_liquidity(
            &mut app,
            mock_vault_config.clone(),
            0.05
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_liquidity_result.units.is_zero());

        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_liquidity_result.to_account.clone(),
                u: mock_send_liquidity_result.units.clone(),
                escrow_amount: mock_send_liquidity_result.swap_amount,
                block_number_mod: app.block_info().height as u32,
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity has increased by the received amount.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity + mock_send_liquidity_result.units
        );



        // Tested action 2: very large send liquidity success (saturate the limit capacity)
        let mock_send_liquidity_result = execute_mock_send_liquidity(
            &mut app,
            mock_vault_config.clone(),
            0.5
        );

        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_liquidity_result.to_account.clone(),
                u: mock_send_liquidity_result.units.clone(),
                escrow_amount: mock_send_liquidity_result.swap_amount,
                block_number_mod: app.block_info().height as u32,
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity has increased by the received amount.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.max_limit_capacity     // ! Vault capacity is at the maximum
        );
        
    }


    #[test]
    fn test_security_limit_change_on_send_liquidity_timeout() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.8         // ! Decrease the initial limit capacity by 20%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action: send liquidity failure
        let mock_send_liquidity_result = execute_mock_send_liquidity(
            &mut app,
            mock_vault_config.clone(),
            0.05
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_liquidity_result.units.is_zero());

        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_liquidity_result.to_account.clone(),
                u: mock_send_liquidity_result.units.clone(),
                escrow_amount: mock_send_liquidity_result.swap_amount,
                block_number_mod: app.block_info().height as u32,
            },
            &[]
        ).unwrap();



        // Make sure the security limit capacity has not changed.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );
        
    }


    #[test]
    fn test_security_limit_change_on_receive_liquidity() {

        let mut app = App::default();
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut app,
            0.8         // ! Decrease the initial limit capacity by 20%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action 1: small receive liquidity
        let received_units_percentage = 0.05;
        let units = f64_to_u256(
            u256_to_f64(mock_vault_config.current_limit_capacity) * received_units_percentage
        ).unwrap();
        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_account: SETUP_MASTER.to_string(),
                u: units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity has decreased by the received amount.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity - units
        );



        // Tested action 2: too large receive liquidity
        let units = observed_limit_capacity + U256::one();  // ! One unit more than allowed.
        let response_result = app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_account: SETUP_MASTER.to_string(),
                u: units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SecurityLimitExceeded { overflow }
                if overflow == U256::one()
        ));



        // Tested action 3: max receive liquidity
        let units = observed_limit_capacity;
        app.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &VolatileExecuteMsg::ReceiveLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_account: SETUP_MASTER.to_string(),
                u: units,
                min_vault_tokens: Uint128::zero(),
                min_reference_asset: Uint128::zero(),
                from_amount: U256::zero(),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            &[]
        ).unwrap();

        // Make sure the security limit capacity is at zero.
        let observed_limit_capacity = query_limit_capacity(&mut app, vault.clone());
        assert!(observed_limit_capacity.is_zero());
        
    }

}