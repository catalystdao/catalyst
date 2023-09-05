mod test_volatile_weights_update {
    use std::f64::consts::LN_2;

    use cosmwasm_std::{Uint128, Addr, Attribute, Timestamp};
    use catalyst_vault_common::{ContractError, msg::{WeightResponse, GetLimitCapacityResponse}, event::format_vec_for_event, asset::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128, u256_to_f64}, definitions::{SETUP_MASTER, FACTORY_OWNER}, contract::mock_factory_deploy_vault, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::{TestEnv, TestAsset};
    use crate::{msg::{VolatileExecuteMsg, VolatileExecuteExtension, QueryMsg, TargetWeightResponse, WeightsUpdateFinishTimestampResponse}, tests::{helpers::volatile_vault_contract_storage, parameters::AMPLIFICATION}};


    // Test helpers

    fn set_mock_vault(
        env: &mut TestEnv,
        vault_assets: Vec<TestAsset>,
        initial_vault_weights: Vec<Uint128>
    ) -> Addr {

        // Instantiate and initialize vault
        let vault_balances: Vec<Uint128> = vault_assets.iter()
            .map(|_| Uint128::new(100000u128))
            .collect();

        let vault_code_id = volatile_vault_contract_storage(env.get_app());

        mock_factory_deploy_vault::<Asset, _, _>(
            env,
            vault_assets,
            vault_balances.clone(),
            initial_vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            None,
            None
        )

    }

    /// Trigger an interal `update_weights()` by executing a zero-valued local swap.
    fn trigger_weights_update(
        env: &mut TestEnv,
        vault: Addr,
        new_timestamp: Timestamp
    ) {

        // Set the new block timestamp
        env.get_app().update_block(|block| {
            block.time = new_timestamp;
            block.height += 1;
        });

        // Execute the local swap
        let vault_assets = env.get_assets();
        
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &VolatileExecuteMsg::LocalSwap {
                from_asset_ref: vault_assets[0].get_asset_ref(),
                to_asset_ref: vault_assets[1].get_asset_ref(),
                amount: Uint128::zero(),
                min_out: Uint128::zero()
            },
            vec![vault_assets[0].clone()],
            vec![Uint128::zero()]
        ).unwrap();
    }



    // Tests

    #[test]
    fn test_set_weights() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets.clone(),
            initial_vault_weights.clone()
        );

        // Define the new weights and the target time
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| f64_to_uint128(uint128_to_f64(*weight) * 1.25).unwrap())  // Set larger weights than the current ones
            .collect();
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action: set weights
        let response = env.execute_contract(
            Addr::unchecked(Addr::unchecked(FACTORY_OWNER)),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap();



        // Check the response event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-weights");

        assert_eq!(
            event.attributes[1],
            Attribute::new("target_timestamp", target_timestamp.to_string())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("target_weights", format_vec_for_event(new_weights.clone()))
        );


        // Check that the target weights are set
        vault_assets.iter()
            .enumerate()
            .for_each(|(i, asset)| {

                let queried_target_weight = env.get_app().wrap().query_wasm_smart::<TargetWeightResponse>(
                    vault.clone(),
                    &QueryMsg::TargetWeight { asset_ref: asset.get_asset_ref() }
                ).unwrap().target_weight;

                assert_eq!(
                    queried_target_weight,
                    new_weights[i]
                )
            });

        // Check that the update target timestamp is set
        let queried_target_timestamp = env.get_app().wrap().query_wasm_smart::<WeightsUpdateFinishTimestampResponse>(
            vault.clone(),
            &QueryMsg::WeightsUpdateFinishTimestamp {}
        ).unwrap().timestamp;

        assert_eq!(
            queried_target_timestamp.u64(),
            target_timestamp
        );

    }


    #[test]
    fn test_set_zero_weights() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets,
            initial_vault_weights.clone()
        );

        // Define the new weights and the target time
        let new_weights: Vec<Uint128> = vec![
            Uint128::from(2000u128),
            Uint128::from(300000u128),
            Uint128::zero()             // ! Third weight set to 0
        ];
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action: set weights
        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidWeight {}
        ));

    }


    #[test]
    fn test_set_too_large_weight_change() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets,
            initial_vault_weights.clone()
        );

        // Define the new weights and the target time
        let max_adjustment_factor = 10.;
        let too_large_adjustment_factor = max_adjustment_factor + 0.1;

        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action 1: set weights max increase works
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| {
                f64_to_uint128(uint128_to_f64(*weight) * max_adjustment_factor).unwrap()
            })
            .collect();

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // Make sure the transaction succeeds



        // Tested action 2: set weights too large fails
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| {
                f64_to_uint128(uint128_to_f64(*weight) * too_large_adjustment_factor).unwrap()
            })
            .collect();

        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidWeight {}
        ));



        // Tested action 3: set weights max decrease works
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| {
                f64_to_uint128(uint128_to_f64(*weight) / max_adjustment_factor).unwrap()
            })
            .collect();

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // Make sure the transaction succeeds



        // Tested action 4: set weights too small fails
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| {
                f64_to_uint128(uint128_to_f64(*weight) / too_large_adjustment_factor).unwrap()
            })
            .collect();

        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidWeight {}
        ))

    }


    #[test]
    fn test_set_weight_invalid_target_timestamp() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets,
            initial_vault_weights.clone()
        );

        // Define the new weights and the target time
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| f64_to_uint128(uint128_to_f64(*weight) * 1.25).unwrap())
            .collect();



        // Tested action 1: minimum adjustment time works
        let min_adjustment_time_seconds = 7*24*60*60;   // 7 days
        let target_timestamp = env.get_app().block_info().time.seconds() + min_adjustment_time_seconds;
        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // Make sure the transaction succeeds



        // Tested action 2: too small adjustment time fails
        let target_timestamp = env.get_app().block_info().time.seconds()
            + min_adjustment_time_seconds
            - 1;   // ! 1 second less than allowed

        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidTargetTime {}
        ));



        // Tested action 3: maximum adjustment time works
        let max_adjustment_time_seconds = 365*24*60*60;   // 365 days
        let target_timestamp = env.get_app().block_info().time.seconds() + max_adjustment_time_seconds;
        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // Make sure the transaction succeeds



        // Tested action 2: too large adjustment time fails
        let target_timestamp = env.get_app().block_info().time.seconds()
            + max_adjustment_time_seconds
            + 1;   // ! 1 second more than allowed

        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidTargetTime {}
        ));

    }


    #[test]
    fn test_set_weight_invalid_count() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets,
            initial_vault_weights.clone()
        );

        // Define the new weights and the target time
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action 1: no weights
        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: vec![] // ! No weights
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == String::from("Invalid weights count.")
        ));



        // Tested action 2: too few weights
        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: vec![
                        Uint128::from(2000u128),
                        Uint128::from(300000u128)
                    ]   // ! One-too-few weights
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == String::from("Invalid weights count.")
        ));



        // Tested action 2: too many weights
        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: vec![
                        Uint128::from(2000u128),
                        Uint128::from(300000u128),
                        Uint128::from(500000u128),
                        Uint128::from(600000u128)   // ! One-too-many weights
                    ]
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidParameters { reason }
                if reason == String::from("Invalid weights count.")
        ));

    }


    #[test]
    fn test_set_weight_unauthorized() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets,
            initial_vault_weights.clone()
        );

        // Define the new weights and the target time
        let new_weights: Vec<Uint128> = initial_vault_weights.iter()
            .map(|weight| f64_to_uint128(uint128_to_f64(*weight) * 1.25).unwrap())
            .collect();
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action: set weights
        let response_result = env.execute_contract(
            Addr::unchecked("not-factory-owner"),   // ! Not the factory owner
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
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
    fn test_weights_update_calculation() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets.clone(),
            initial_vault_weights.clone()
        );

        // Define and set the new weights and the target time
        let new_weights: Vec<Uint128> = vec![
            Uint128::from(7777u128),    // First weight increases
            Uint128::from(222222u128),  // Second weight decreases
            Uint128::from(500000u128)   // Third weight does not change
        ];
        let update_duration = 30*24*60*60;   // 30 days
        let start_timestamp = env.get_app().block_info().time.seconds();
        let target_timestamp = start_timestamp + update_duration;

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap();



        // Test action: Trigger update at different stages of the update and verify the weights
        let update_checks = vec![
            0.,
            0.25,
            0.25,   // ! Trigger an update twice on the same block
            0.80,
            1.,
            1.1     // ! Make sure the weights update stops after 100%
        ];

        update_checks
            .iter()
            .for_each(|update_progress| {
                
                // Execute a local swap to trigger a weights recomputation
                trigger_weights_update(
                    &mut env,
                    vault.clone(),
                    Timestamp::from_seconds(
                        start_timestamp
                        + (update_duration as f64 * update_progress) as u64
                    )
                );

                // Verify that the weights are set correctly
                vault_assets.iter()
                    .enumerate()
                    .for_each(|(i, asset)| {

                        let queried_current_weight = env.get_app().wrap().query_wasm_smart::<WeightResponse>(
                            vault.clone(),
                            &QueryMsg::Weight { asset_ref: asset.get_asset_ref() }
                        ).unwrap().weight;

                        let initial_weight = initial_vault_weights[i].u128() as f64;
                        let target_weight = new_weights[i].u128() as f64;
                        let expected_current_weight = initial_weight + (target_weight - initial_weight) * update_progress.min(1.);

                        assert!(uint128_to_f64(queried_current_weight) <= expected_current_weight * 1.01);
                        assert!(uint128_to_f64(queried_current_weight) >= expected_current_weight * 0.99);
                    });
            })

    }


    #[test]
    fn test_weights_update_finish() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets.clone(),
            initial_vault_weights.clone()
        );

        // Define and set the new weights and the target time
        let new_weights: Vec<Uint128> = vec![
            Uint128::from(7777u128),    // First weight increases
            Uint128::from(222222u128),  // Second weight decreases
            Uint128::from(500000u128)   // Third weight does not change
        ];
        let update_duration = 30*24*60*60;   // 30 days
        let target_timestamp = env.get_app().block_info().time.seconds() + update_duration;

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap();



        // Test action: Trigger the 'update finish' logic of the weights update

        // Execute a local swap to trigger a weights recomputation
        trigger_weights_update(
            &mut env,
            vault.clone(),
            Timestamp::from_seconds(
                target_timestamp        // ! Set the block time to the 'weights update' finish timestamp
            )
        );

        // Verify that the weights are set correctly
        vault_assets.iter()
            .enumerate()
            .for_each(|(i, asset)| {

                let queried_current_weight = env.get_app().wrap().query_wasm_smart::<WeightResponse>(
                    vault.clone(),
                    &QueryMsg::Weight { asset_ref: asset.get_asset_ref() }
                ).unwrap().weight;

                assert_eq!(
                    queried_current_weight,
                    new_weights[i]
                );
            });

        // Check that the time variables have been cleared
        let queried_target_timestamp = env.get_app().wrap().query_wasm_smart::<WeightsUpdateFinishTimestampResponse>(
            vault.clone(),
            &QueryMsg::WeightsUpdateFinishTimestamp {}
        ).unwrap().timestamp;

        assert_eq!(
            queried_target_timestamp.u64(),
            0u64
        );
    }


    #[test]
    fn test_weights_update_security_limit() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let vault_assets = env.get_assets()[..3].to_vec();
        let initial_vault_weights = vec![Uint128::from(2000u128), Uint128::from(300000u128), Uint128::from(500000u128)];
        let vault = set_mock_vault(
            &mut env,
            vault_assets.clone(),
            initial_vault_weights.clone()
        );

        // Define and set the new weights and the target time
        let new_weights: Vec<Uint128> = vec![
            Uint128::from(7777u128),    // First weight increases
            Uint128::from(222222u128),  // Second weight decreases
            Uint128::from(500000u128)   // Third weight does not change
        ];
        let update_duration = 30*24*60*60;   // 30 days
        let start_timestamp = env.get_app().block_info().time.seconds();
        let target_timestamp = start_timestamp + update_duration;

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &VolatileExecuteMsg::Custom(
                VolatileExecuteExtension::SetWeights {
                    target_timestamp: target_timestamp.into(),
                    new_weights: new_weights.clone()
                }
            ),
            vec![],
            vec![]
        ).unwrap();



        // Test action: Trigger update at different stages of the update and verify the weights
        let update_checks = vec![
            0.,
            0.25,
            0.25,   // ! Trigger an update twice on the same block
            0.80,
            1.,
            1.1     // ! Make sure the update stops after 100%
        ];

        update_checks
            .iter()
            .for_each(|update_progress| {
                
                // Execute a local swap to trigger a weights recomputation
                trigger_weights_update(
                    &mut env,
                    vault.clone(),
                    Timestamp::from_seconds(
                        start_timestamp
                        + (update_duration as f64 * update_progress) as u64
                    )
                );

                // Verify that the security limit is correct
                let current_weights = vault_assets.iter()
                    .map(|asset| {

                        env.get_app().wrap().query_wasm_smart::<WeightResponse>(
                            vault.clone(),
                            &QueryMsg::Weight { asset_ref: asset.get_asset_ref() }
                        ).unwrap().weight
    
                    })
                    .collect::<Vec<Uint128>>();

                let expected_limit_capacity = uint128_to_f64(current_weights.iter().sum())
                    * LN_2
                    * 1e18; // Multiplied by 1e18 as the queried limit capacity is in WAD notation
                
                let queried_limit_capacity = u256_to_f64(
                        env.get_app().wrap().query_wasm_smart::<GetLimitCapacityResponse>(
                        vault.clone(),
                        &QueryMsg::GetLimitCapacity {}
                    ).unwrap().capacity
                );
        
                assert!(queried_limit_capacity <= expected_limit_capacity * 1.000001);
                assert!(queried_limit_capacity >= expected_limit_capacity * 0.999999);
            })
    }

}