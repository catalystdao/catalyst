mod test_amplified_security_limit {
    use std::{ops::Div, str::FromStr};

    use catalyst_types::{U256, u256, I256};
    use catalyst_vault_common::{msg::{GetLimitCapacityResponse, TotalEscrowedAssetResponse}, ContractError, state::{INITIAL_MINT_AMOUNT, DECAY_RATE}, bindings::Asset};
    use cosmwasm_std::{Addr, Uint128, Binary, Uint64};
    use test_helpers::{contract::{mock_factory_deploy_vault, mock_instantiate_interface, mock_set_vault_connection}, definitions::{SETUP_MASTER, SWAPPER_B, CHANNEL_ID, SWAPPER_C, FACTORY_OWNER}, math::{uint128_to_f64, f64_to_uint128, u256_to_f64, f64_to_u256}, misc::{encode_payload_address, get_response_attribute}, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::{TestEnv, TestAsset, TestApp};
    use crate::{tests::{parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT, AMPLIFICATION}, helpers::amplified_vault_contract_storage}, msg::{QueryMsg, AmplifiedExecuteMsg, AmplifiedExecuteExtension}};


    pub const REMOTE_VAULT: &str = "remote_vault_addr";



    // Test helpers *******************************************************************************

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MockVaultConfig {
        interface: Addr,
        vault: Addr,
        assets: Vec<TestAsset>,
        weights: Vec<Uint128>,
        vault_initial_balances: Vec<Uint128>,
        remote_vault: Binary,
        max_limit_capacity: U256,
        current_limit_capacity: U256
    }

    fn set_mock_vault(
        env: &mut TestEnv,
        bias_limit_capacity: f64
    ) -> MockVaultConfig {

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());
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

        // Connect the vault with a mock vault
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            remote_vault.clone(),
            true
        );

        let intitial_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let current_limit_capacity: U256;
        if bias_limit_capacity != 1. {

            // Trigger 'receive asset' for all assets contained by the vault (otherwise it is not 
            // guaranteed that the limit will be reached before the vault gets depleted of assets)
            let one_minus_amp = 1. - (AMPLIFICATION.u64() as f64) / 1e18;
            vault_initial_balances.iter()
                .zip(&vault_weights)
                .enumerate()
                .for_each(|(i, (vault_balance, weight))| {

                    let vault_balance = uint128_to_f64(*vault_balance);
                    let weight = uint128_to_f64(*weight);

                    let units = calc_units_for_limit_change(
                        one_minus_amp,
                        intitial_limit_capacity,
                        (1. - bias_limit_capacity) / TEST_VAULT_ASSET_COUNT as f64,
                        vault_balance,
                        weight
                    );

                    // Perform a 'receive asset' call to bias the limit capacity

                    env.execute_contract(
                        interface.clone(),
                        vault.clone(),
                        &AmplifiedExecuteMsg::ReceiveAsset {
                            channel_id: CHANNEL_ID.to_string(),
                            from_vault: remote_vault.clone(),
                            to_asset_index: i as u8,
                            to_account: SETUP_MASTER.to_string(),
                            u: units,
                            min_out: Uint128::zero(),
                            from_amount: U256::zero(),
                            from_asset: Binary("from_asset".as_bytes().to_vec()),
                            from_block_number_mod: 0u32,
                            calldata_target: None,
                            calldata: None
                        },
                        vec![],
                        vec![]
                    ).unwrap();
                });

            current_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        }
        else {
            current_limit_capacity = intitial_limit_capacity;
        }

        // Recompute the max limit capacity (as it depends on asset balances for amplified vaults)
        let max_limit_capacity = calc_max_limit_capacity(
            env,
            vault.clone(),
            vault_assets.clone(),
            vault_weights.clone()
        );

        MockVaultConfig {
            interface,
            vault,
            assets: vault_assets,
            weights: vault_weights,
            vault_initial_balances,
            remote_vault,
            max_limit_capacity,
            current_limit_capacity
        }

    }

    /// Helper to calculate the 'units' required to decrease the limit capacity by 'decrease_factor'.
    fn calc_units_for_limit_change(
        one_minus_amp: f64,
        limit_capacity: U256,
        decrease_factor: f64,
        vault_balance: f64,
        vault_weight: f64
    ) -> U256 {

        let desired_received_asset_balance = u256_to_f64(limit_capacity)
            * (decrease_factor)
            / vault_weight;

        if desired_received_asset_balance > vault_balance {
            panic!("Unable to calculate units for limit change, not enough vault balance");
        }

        let weighted_balance_ampped = (vault_balance * vault_weight).powf(one_minus_amp);
        
        let units_f64 = (
            1. - (1. - desired_received_asset_balance/vault_balance).powf(one_minus_amp)
        )*weighted_balance_ampped;

        f64_to_u256(units_f64.max(0.) * 1e18).unwrap()

    }

    /// Helper to calculate the theoretical maximum limit capacity based PURELY on the vault's asset balances.
    fn calc_max_limit_capacity(
        env: &mut TestEnv,
        vault: Addr,
        vault_assets: Vec<TestAsset>,
        vault_weights: Vec<Uint128>
    ) -> U256 {
        vault_assets.iter()
            .zip(vault_weights)
            .fold(U256::zero(), |acc, (asset, weight)| {

                let vault_balance = asset.query_balance(
                    env.get_app(),
                    vault.to_string()
                );

                let escrowed_balance = env.get_app().wrap().query_wasm_smart::<TotalEscrowedAssetResponse>(
                    vault.clone(),
                    &QueryMsg::TotalEscrowedAsset { asset_ref: asset.get_asset_ref() }
                ).unwrap().amount;

                let effective_balance = vault_balance - escrowed_balance;

                acc + U256::from(effective_balance) * U256::from(weight)
            })
            .div(u256!("2"))
    }


    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MockSendAsset {
        to_account: Binary,
        from_asset: TestAsset,
        from_weight: Uint128,
        swap_amount: Uint128,
        units: U256,
        fee: Uint128
    }

    fn execute_mock_send_asset(
        env: &mut TestEnv,
        mock_vault_config: MockVaultConfig,
        send_percentage: f64
    ) -> MockSendAsset {

        let from_asset_idx = 0;
        let from_asset = mock_vault_config.assets[from_asset_idx].clone();
        let from_weight = mock_vault_config.weights[from_asset_idx].clone();
        let from_balance = mock_vault_config.vault_initial_balances[from_asset_idx];
        let swap_amount = f64_to_uint128(uint128_to_f64(from_balance) * send_percentage).unwrap();

        let to_asset_idx = 1;
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Execute send asset
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            mock_vault_config.vault.clone(),
            &AmplifiedExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            vec![from_asset.clone()],
            vec![swap_amount]
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
            from_weight,
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
        env: &mut TestEnv,
        mock_vault_config: MockVaultConfig,
        send_percentage: f64
    ) -> MockSendLiquidity {

        let swap_amount = f64_to_uint128(
            uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage
        ).unwrap();

        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Execute send liquidity
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            mock_vault_config.vault.clone(),
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            vec![],
            vec![]
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
        app: &mut TestApp,
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

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = amplified_vault_contract_storage(env.get_app());



        // Tested action: intialize a new vault
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            &mut env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(interface.clone()),
            None,
            None
        );



        // Check the limit capacity
        let queried_limit_capacity = u256_to_f64(query_limit_capacity(env.get_app(), vault.clone()));

        let expected_limit_capacity = vault_initial_balances.iter()
            .zip(&vault_weights)
            .fold(0., |acc, (balance, weight)| -> f64 {
                let balance = uint128_to_f64(*balance);
                let weight = uint128_to_f64(*weight);

                acc + balance * weight
            })
            .div_euclid(2.);

        assert!(queried_limit_capacity <= expected_limit_capacity * 1.000001);
        assert!(queried_limit_capacity >= expected_limit_capacity * 0.999999);

    }


    #[test]
    fn test_security_limit_decay() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.         // ! Decrease the initial limit capacity by 100%
        );
        let vault = mock_vault_config.vault.clone();

        // Make sure the capacity is close to zero
        let relative_capacity = u256_to_f64(mock_vault_config.current_limit_capacity)
            / u256_to_f64(mock_vault_config.max_limit_capacity);
        assert!(relative_capacity < 0.001);

        // Check the capacity calculation at different intervals
        let start_timestamp = env.get_app().block_info().time;
        let check_steps = vec![0.2, 0.7, 1., 1.1];

        check_steps.iter().for_each(|step| {

            let time_elapsed = (u256_to_f64(DECAY_RATE) * step) as u64;
            env.get_app().update_block(|block| {
                block.time = start_timestamp.plus_seconds(time_elapsed);
            });

            let queried_capacity = u256_to_f64(
                query_limit_capacity(env.get_app(), vault.clone())
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

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.7         // ! Decrease the initial limit capacity by 30%
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



        // Tested action: send asset
        let remote_vault = encode_payload_address(REMOTE_VAULT.as_bytes());
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::SendAsset {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index: to_asset_idx,
                amount: swap_amount,
                min_out: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();



        // Make sure some assets have been sent (i.e. that the test is correctly setup)
        assert!(!swap_amount.is_zero());

        // Make sure the security limit has not increased (as it does not get increased until the ACK
        // is received)
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        )
        
    }


    #[test]
    fn test_security_limit_change_on_send_asset_success() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.7         // ! Decrease the initial limit capacity by 30%
        );
        let vault = mock_vault_config.vault.clone();

        // Keep track of the expected maximum limit capacity, as it will change as assets flow
        // into the vault.
        let mut expected_max_limit_capacity = mock_vault_config.max_limit_capacity;



        // Tested action 1: small send asset success
        let mock_send_asset_result = execute_mock_send_asset(
            &mut env,
            mock_vault_config.clone(),
            0.05    // ! Small swap
        );
        // Make sure some assets have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_asset_result.swap_amount.is_zero());

        let effective_swap_amount = mock_send_asset_result.swap_amount - mock_send_asset_result.fee;
        expected_max_limit_capacity += U256::from(effective_swap_amount * mock_send_asset_result.from_weight)/u256!("2");

        let block_number_mod = env.get_app().block_info().height as u32;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: effective_swap_amount,
                asset_ref: mock_send_asset_result.from_asset.get_asset_ref(),
                block_number_mod
            },
            vec![],
            vec![]
        ).unwrap();

        // Make sure the security limit capacity has increased by the received amount.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        let expected_limit_capacity = mock_vault_config.current_limit_capacity
            + U256::from(effective_swap_amount)*U256::from(mock_send_asset_result.from_weight);
        assert_eq!(
            observed_limit_capacity,
            expected_limit_capacity
        );



        // Tested action 2: very large send asset success (saturate the limit capacity)
        let mock_send_asset_result = execute_mock_send_asset(
            &mut env,
            mock_vault_config.clone(),
            1.  // ! Very large swap
        );

        let effective_swap_amount = mock_send_asset_result.swap_amount - mock_send_asset_result.fee;
        expected_max_limit_capacity += U256::from(effective_swap_amount * mock_send_asset_result.from_weight)/u256!("2");

        let block_number_mod = env.get_app().block_info().height as u32;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: effective_swap_amount,
                asset_ref: mock_send_asset_result.from_asset.get_asset_ref(),
                block_number_mod
            },
            vec![],
            vec![]
        ).unwrap();

        // Make sure the security limit capacity is at the expected maximum.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        assert_eq!(
            observed_limit_capacity,
            expected_max_limit_capacity // ! Make sure the limit capacity as at its maximum
        );

    }


    #[test]
    fn test_security_limit_change_on_send_asset_timeout() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.7         // ! Decrease the initial limit capacity by 30%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action: send asset failure
        let mock_send_asset_result = execute_mock_send_asset(
            &mut env,
            mock_vault_config.clone(),
            0.05
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_asset_result.units.is_zero());

        let block_number_mod = env.get_app().block_info().height as u32;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: mock_send_asset_result.swap_amount - mock_send_asset_result.fee,
                asset_ref: mock_send_asset_result.from_asset.get_asset_ref(),
                block_number_mod
            },
            vec![],
            vec![]
        ).unwrap();



        // Make sure the security limit capacity has not changed.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );
        
    }


    #[test]
    fn test_security_limit_change_on_receive_asset() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.1         // ! Decrease the initial limit capacity by 90%
        );
        let vault = mock_vault_config.vault.clone();

        // Keep track of the expected maximum limit capacity, as it will change as assets flow
        // out of the vault.
        let mut expected_max_limit_capacity = mock_vault_config.max_limit_capacity;



        // Tested action 1: small receive asset
        let received_limit_percentage = 0.05;
        let units = calc_units_for_limit_change(
            1. - (AMPLIFICATION.u64() as f64) / 1e18,
            mock_vault_config.current_limit_capacity,
            received_limit_percentage,
            mock_vault_config.vault_initial_balances[0].u128() as f64,
            mock_vault_config.weights[0].u128() as f64
        );

        let response = env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
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
            vec![],
            vec![]
        ).unwrap();

        // Make sure the security limit capacity has decreased by the return amount.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let observed_return = get_response_attribute::<Uint128>(
            response.events[1].clone(),
            "to_amount"
        ).unwrap();
        let expected_limit_change = U256::from(observed_return*mock_vault_config.weights[0]);

        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity - expected_limit_change
        );
        
        expected_max_limit_capacity -= expected_limit_change/u256!("2");




        // Tested action 2: too large receive asset
        let vault_balance = mock_vault_config.assets[0].query_balance(env.get_app(), vault.to_string());
        let max_units = calc_units_for_limit_change(
            1. - (AMPLIFICATION.u64() as f64) / 1e18,
            observed_limit_capacity,
            1.,     // ! Max out the limit
            vault_balance.u128() as f64,
            mock_vault_config.weights[0].u128() as f64
        );

        let units = f64_to_u256(
            1.01 * u256_to_f64(max_units)     // ! Try to receive more than allowed
        ).unwrap();

        let response_result = env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
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
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SecurityLimitExceeded { overflow }
                // Check that the limit capacity overflow is small
                if u256_to_f64(overflow) / u256_to_f64(observed_limit_capacity) < 0.1
        ));



        // Tested action 3: max receive asset
        let response = env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveAsset {
                channel_id: CHANNEL_ID.to_string(),
                from_vault: mock_vault_config.remote_vault.clone(),
                to_asset_index: 0u8,
                to_account: SETUP_MASTER.to_string(),
                u: max_units,
                min_out: Uint128::zero(),
                from_amount: U256::zero(),
                from_asset: Binary("from_asset".as_bytes().to_vec()),
                from_block_number_mod: 0u32,
                calldata_target: None,
                calldata: None
            },
            vec![],
            vec![]
        ).unwrap();

        // Make sure the security limit capacity is close zero.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert!(u256_to_f64(observed_limit_capacity) / u256_to_f64(expected_max_limit_capacity) < 0.01);

        // Verify the max_limit_capacity (increase the block time by DECAY_RATE)
        env.get_app().update_block(|block| block.time = block.time.plus_seconds(DECAY_RATE.as_u64()));

        let observed_return = get_response_attribute::<Uint128>(
            response.events[1].clone(),
            "to_amount"
        ).unwrap();
        let expected_limit_change = U256::from(observed_return*mock_vault_config.weights[0]);

        expected_max_limit_capacity -= expected_limit_change/u256!("2");

        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert_eq!(
            observed_limit_capacity,
            expected_max_limit_capacity
        )

    }


    #[test]
    fn test_security_limit_max_capacity_recalculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            1.
        );
        let vault = mock_vault_config.vault.clone();

        // Execute a send asset
        let mock_send_asset_result = execute_mock_send_asset(
            &mut env,
            mock_vault_config.clone(),
            0.05
        );
        // Make sure some assets have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_asset_result.swap_amount.is_zero());



        // Tested action 1: Recalculate the max capacity BEFORE 'success' is received
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::UpdateMaxLimitCapacity {}
            ),
            vec![],
            vec![]
        ).unwrap();

        // Make sure the max limit capacity has only increased by the swap's FEE amount (minus the gov fee!),
        // as the effective swap amount may be returned to the swapper in the case of an unsuccessful swap.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let observed_fee = mock_send_asset_result.fee;
        let observed_gov_fee = mock_vault_config.assets[0].query_balance(
            env.get_app(),
            FACTORY_OWNER.to_string()
        );
        let expected_limit_capacity = mock_vault_config.max_limit_capacity
            + U256::from((observed_fee-observed_gov_fee)*mock_send_asset_result.from_weight)/u256!("2");

        assert_eq!(
            observed_limit_capacity,
            expected_limit_capacity
        );



        // Execute swap success
        let effective_swap_amount = mock_send_asset_result.swap_amount - mock_send_asset_result.fee;
        let block_number_mod = env.get_app().block_info().height as u32;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::OnSendAssetSuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_asset_result.to_account.clone(),
                u: mock_send_asset_result.units.clone(),
                escrow_amount: effective_swap_amount,
                asset_ref: mock_send_asset_result.from_asset.get_asset_ref(),
                block_number_mod
            },
            vec![],
            vec![]
        ).unwrap();



        // Tested action 2: Recalculate the max capacity AFTER 'success' is received
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::UpdateMaxLimitCapacity {}
            ),
            vec![],
            vec![]
        ).unwrap();

        // Make sure the max limit capacity has increased by the TOTAL swap amount (minus the gov fee!).
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        let expected_limit_capacity = mock_vault_config.max_limit_capacity
            + U256::from(
                (mock_send_asset_result.swap_amount-observed_gov_fee)*mock_send_asset_result.from_weight
            )/u256!("2");

        assert_eq!(
            observed_limit_capacity,
            expected_limit_capacity
        );

    }


    // Liquidity swap tests ***********************************************************************

    #[test]
    fn test_security_limit_change_on_send_liquidity() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.8         // ! Decrease the initial limit capacity by 20%
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
        let response = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID.to_string(),
                to_vault: remote_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![])
            },
            vec![],
            vec![]
        ).unwrap();



        // Make sure some units have been sent (i.e. that the test is correctly setup)
        let observed_units = get_response_attribute::<U256>(
            response.events[1].clone(),
            "units"
        ).unwrap();
        assert!(!observed_units.is_zero());

        // Make sure the security limit has not increased.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        )
        
    }


    #[test]
    fn test_security_limit_change_on_send_liquidity_success() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.8         // ! Decrease the initial limit capacity by 20%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action: send liquidity success
        let mock_send_liquidity_result = execute_mock_send_liquidity(
            &mut env,
            mock_vault_config.clone(),
            0.05
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_liquidity_result.units.is_zero());

        let block_number_mod = env.get_app().block_info().height as u32;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::OnSendLiquiditySuccess {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_liquidity_result.to_account.clone(),
                u: mock_send_liquidity_result.units.clone(),
                escrow_amount: mock_send_liquidity_result.swap_amount,
                block_number_mod
            },
            vec![],
            vec![]
        ).unwrap();

        // Check the security limit has not increased. (Quirk of the amplified vault)
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );
        
    }


    #[test]
    fn test_security_limit_change_on_send_liquidity_timeout() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.8         // ! Decrease the initial limit capacity by 20%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action: send liquidity failure
        let mock_send_liquidity_result = execute_mock_send_liquidity(
            &mut env,
            mock_vault_config.clone(),
            0.05
        );
        // Make sure some units have been sent (i.e. that the test is correctly setup)
        assert!(!mock_send_liquidity_result.units.is_zero());

        let block_number_mod = env.get_app().block_info().height as u32;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::OnSendLiquidityFailure {
                channel_id: CHANNEL_ID.to_string(),
                to_account: mock_send_liquidity_result.to_account.clone(),
                u: mock_send_liquidity_result.units.clone(),
                escrow_amount: mock_send_liquidity_result.swap_amount,
                block_number_mod
            },
            vec![],
            vec![]
        ).unwrap();



        // Make sure the security limit capacity has not changed.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );
        
    }


    #[test]
    #[ignore]   // TODO review
    fn test_security_limit_change_on_receive_liquidity() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.1         // ! Decrease the initial limit capacity by 20%
        );
        let vault = mock_vault_config.vault.clone();



        // Tested action 1: small receive liquidity
        let received_limit_percentage = 0.05;
        let units = calc_units_for_limit_change(
            1. - (AMPLIFICATION.u64() as f64) / 1e18,
            mock_vault_config.current_limit_capacity,
            received_limit_percentage,
            mock_vault_config.vault_initial_balances[0].u128() as f64,
            mock_vault_config.weights[0].u128() as f64
        );
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveLiquidity {
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
            vec![],
            vec![]
        ).unwrap();

        // Make sure the security limit capacity has decreased.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert!(
            observed_limit_capacity < mock_vault_config.current_limit_capacity
        );

        // Tested action 2: too large receive liquidity
        let received_limit_percentage = 1.; // Max
        let max_units = calc_units_for_limit_change(
            1. - (AMPLIFICATION.u64() as f64) / 1e18,
            observed_limit_capacity,
            received_limit_percentage,
            mock_vault_config.vault_initial_balances[0].u128() as f64,
            mock_vault_config.weights[0].u128() as f64
        );
        let units = f64_to_u256(
            1.01 * u256_to_f64(max_units)
        ).unwrap();
        let response_result = env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveLiquidity {
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
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::SecurityLimitExceeded { overflow }
                if overflow == U256::one()
        ));



        // Tested action 3: max receive liquidity
        let units = observed_limit_capacity;
        env.execute_contract(
            mock_vault_config.interface.clone(),
            vault.clone(),
            &AmplifiedExecuteMsg::ReceiveLiquidity {
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
            vec![],
            vec![]
        ).unwrap();

        // Make sure the security limit capacity is at zero.
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());
        assert!(observed_limit_capacity.is_zero());
        
    }


    // Local swap tests ***************************************************************************
    
    #[test]
    fn test_security_limit_change_on_local_swap() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            1.
        );
        let vault = mock_vault_config.vault.clone();

        // Define local swap config
        let from_asset_idx = 0;
        let from_asset = mock_vault_config.assets[from_asset_idx].clone();
        let from_weight = mock_vault_config.weights[from_asset_idx];
        let from_balance = mock_vault_config.vault_initial_balances[from_asset_idx];

        let to_asset_idx = 1;
        let to_asset = mock_vault_config.assets[to_asset_idx].clone();
        let to_weight = mock_vault_config.weights[to_asset_idx];

        // Swap 25% of the vault one way
        let swap_amount = from_balance * Uint128::from(25u64)/ Uint128::from(100u64);



        // NOTE: The current implementation of local swaps does not adjust the used limit capacity, but
        // rather only the max limit capacity. This will result in the available limit capacity changing
        // after local swaps.



        // Tested action 1: local swap in one direction
        let result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::LocalSwap {
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(),
                amount: swap_amount,
                min_out: Uint128::zero()
            },
            vec![from_asset.clone()],
            vec![swap_amount]
        ).unwrap();

        // Verify the max limit capacity has changed
        let observed_max_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let first_observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "to_amount"
        ).unwrap();

        let first_expected_max_limit_capacity_change = (
            I256::from(swap_amount * from_weight)
            - I256::from(first_observed_return * to_weight)
        ).div(I256::from(2u64));

        let expected_max_limit_capacity = (
            mock_vault_config.max_limit_capacity.as_i256()
                + first_expected_max_limit_capacity_change
        ).as_u256();

        assert_eq!(
            observed_max_limit_capacity,
            expected_max_limit_capacity
        );



        // Tested action 2: local swap in the opposite direction
        let result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::LocalSwap {
                from_asset_ref: to_asset.get_asset_ref(),
                to_asset_ref: from_asset.get_asset_ref(),
                amount: first_observed_return,
                min_out: Uint128::zero()
            },
            vec![to_asset.clone()],
            vec![first_observed_return]
        ).unwrap();

        // Verify the max limit capacity has changed
        let observed_max_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let second_observed_return = get_response_attribute::<Uint128>(
            result.events[1].clone(),
            "to_amount"
        ).unwrap();

        let second_expected_max_limit_capacity_change = (
            I256::from(first_observed_return * to_weight)
            - I256::from(second_observed_return * from_weight)
        ).div(I256::from(2u64));

        let expected_max_limit_capacity = (
            mock_vault_config.max_limit_capacity.as_i256()
            + first_expected_max_limit_capacity_change
            + second_expected_max_limit_capacity_change
        ).as_u256();

        assert_eq!(
            observed_max_limit_capacity,
            expected_max_limit_capacity
        );

    }


    // Deposit and Withdrawals tests **************************************************************
    
    #[test]
    fn test_security_limit_change_on_deposit() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.6         // ! Decrease the initial limit capacity by 40%
        );
        let vault = mock_vault_config.vault.clone();
        
        // Define deposit config
        let deposit_percentage = 0.15;
        let deposit_amounts: Vec<Uint128> = mock_vault_config.vault_initial_balances.iter()
            .map(|vault_balance| {
                f64_to_uint128(
                    uint128_to_f64(*vault_balance) * deposit_percentage
                ).unwrap()
            }).collect();

        // Tested action: deposit
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::DepositMixed {
                deposit_amounts: deposit_amounts.clone(),
                min_out: Uint128::zero()
            },
            mock_vault_config.assets.clone(),
            deposit_amounts.clone()
        ).unwrap();



        // Verify the security limit capacity has not changed
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );

        // Verify the max limit capacity has increased
        env.get_app().update_block(|block| block.time = block.time.plus_seconds(DECAY_RATE.as_u64()));    // Increase block time
        let observed_max_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let expected_max_limit_capacity_change = deposit_amounts.iter()
            .zip(mock_vault_config.weights)
            .fold(U256::zero(), |acc, (amount, weight)| {
                acc + U256::from(amount*weight)
            })
            .div(U256::from(2u64));
        let expected_max_limit_capacity = mock_vault_config.max_limit_capacity + expected_max_limit_capacity_change;

        assert_eq!(
            observed_max_limit_capacity,
            expected_max_limit_capacity
        );

    }
    

    #[test]
    fn test_security_limit_change_on_withdraw_mixed() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.6         // ! Decrease the initial limit capacity by 40%
        );
        let vault = mock_vault_config.vault.clone();
        
        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();
        let withdraw_ratio_f64 = vec![0.5, 0.2, 1.][3-TEST_VAULT_ASSET_COUNT..].to_vec();
        let withdraw_ratio = withdraw_ratio_f64.iter()
            .map(|val| ((val * 1e18) as u64).into()).collect::<Vec<Uint64>>();



        // Tested action: withdraw mixed
        let result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawMixed {
                vault_tokens: withdraw_amount,
                withdraw_ratio: withdraw_ratio.clone(),
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the security limit capacity has not changed
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );

        // Verify the max limit capacity has decreased
        env.get_app().update_block(|block| block.time = block.time.plus_seconds(DECAY_RATE.as_u64()));    // Increase block time
        let observed_max_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let observed_returns = get_response_attribute::<String>(
            result.events[1].clone(),
            "withdraw_amounts"
        ).unwrap()
            .split(", ")
            .map(Uint128::from_str)
            .collect::<Result<Vec<Uint128>, _>>()
            .unwrap();

        let expected_max_limit_capacity_change = observed_returns.iter()
            .zip(mock_vault_config.weights)
            .fold(U256::zero(), |acc, (amount, weight)| {
                acc + U256::from(amount*weight)
            })
            .div(U256::from(2u64));
        let expected_max_limit_capacity = mock_vault_config.max_limit_capacity - expected_max_limit_capacity_change;

        assert_eq!(
            observed_max_limit_capacity,
            expected_max_limit_capacity
        );

    }
    

    #[test]
    fn test_security_limit_change_on_withdraw_all() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());
        
        // Instantiate and initialize vault
        let mock_vault_config = set_mock_vault(
            &mut env,
            0.6         // ! Decrease the initial limit capacity by 40%
        );
        let vault = mock_vault_config.vault.clone();
        
        // Define withdraw config
        let withdraw_percentage = 0.15;     // Percentage of vault tokens supply
        let withdraw_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * withdraw_percentage).unwrap();



        // Tested action: withdraw all
        let result = env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::WithdrawAll {
                vault_tokens: withdraw_amount,
                min_out: vec![Uint128::zero(); TEST_VAULT_ASSET_COUNT]
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the security limit capacity has not changed
        let observed_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        assert_eq!(
            observed_limit_capacity,
            mock_vault_config.current_limit_capacity
        );

        // Verify the max limit capacity has decreased
        env.get_app().update_block(|block| block.time = block.time.plus_seconds(DECAY_RATE.as_u64()));    // Increase block time
        let observed_max_limit_capacity = query_limit_capacity(env.get_app(), vault.clone());

        let observed_returns = get_response_attribute::<String>(
            result.events[1].clone(),
            "withdraw_amounts"
        ).unwrap()
            .split(", ")
            .map(Uint128::from_str)
            .collect::<Result<Vec<Uint128>, _>>()
            .unwrap();

        let expected_max_limit_capacity_change = observed_returns.iter()
            .zip(mock_vault_config.weights)
            .fold(U256::zero(), |acc, (amount, weight)| {
                acc + U256::from(amount*weight)
            })
            .div(U256::from(2u64));
        let expected_max_limit_capacity = mock_vault_config.max_limit_capacity - expected_max_limit_capacity_change;
 
        // NOTE: because of how the weighted-amounts are calculated and used for the security limit adjustment on 
        // `withdraw_all`, rounding errors are introduced which cause the following check not to be exact.
        let observed_max_limit_capacity = u256_to_f64(observed_max_limit_capacity);
        let expected_max_limit_capacity = u256_to_f64(expected_max_limit_capacity);
        assert!(observed_max_limit_capacity <= expected_max_limit_capacity * 1.000001);
        assert!(observed_max_limit_capacity >= expected_max_limit_capacity * 0.999999);

    }

}