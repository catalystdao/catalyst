mod test_amplified_amplification_update {
    use cosmwasm_std::{Uint128, Addr, Attribute, Timestamp, Uint64};
    use catalyst_vault_common::ContractError;
    use test_helpers::{definitions::{SETUP_MASTER, FACTORY_OWNER, CHAIN_INTERFACE}, contract::mock_factory_deploy_vault, env::CustomTestEnv, asset::CustomTestAsset};

    use crate::tests::TestEnv;
    use crate::{msg::{AmplifiedExecuteMsg, AmplifiedExecuteExtension, QueryMsg, TargetAmplificationResponse, AmplificationUpdateFinishTimestampResponse, AmplificationResponse}, tests::{helpers::amplified_vault_contract_storage, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, TEST_VAULT_ASSET_COUNT}}};


    // Test helpers

    fn set_mock_vault(
        env: &mut TestEnv,
        initial_amplification: Uint64
    ) -> Addr {

        // Instantiate and initialize vault
        let vault_code_id = amplified_vault_contract_storage(env.get_app());

        let test_tokens =  env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        mock_factory_deploy_vault(
            env,
            test_tokens,
            TEST_VAULT_BALANCES.to_vec(),
            TEST_VAULT_WEIGHTS.to_vec(),
            initial_amplification,
            vault_code_id,
            None,
            None    // ! Set the vault WITHOUT a cross-chain interface, as amplification changes are disabled 
                    // ! for cross-chain enabled vaults
        )

    }

    //TODO the following function does not work for native assets, as zero valued transfers are not allowed
    /// Trigger an interal `update_amplification()` by executing a zero-valued local swap.
    fn trigger_amplification_update(
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
        let vault_tokens = env.get_assets();
        
        env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            vault.clone(),
            &AmplifiedExecuteMsg::LocalSwap {
                from_asset: vault_tokens[0].get_asset_ref(),
                to_asset: vault_tokens[1].get_asset_ref(),
                amount: Uint128::zero(),
                min_out: Uint128::zero()
            },
            vec![vault_tokens[0].clone()],
            vec![Uint128::zero()]
        ).unwrap();
    }



    // Tests

    #[test]
    fn test_set_amplification() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 900000000000000000u64;
        let vault = set_mock_vault(
            &mut env,
            initial_amplification.into()
        );

        // Define the new amplification and the target time
        let new_amplification: u64 = ((initial_amplification as f64) * 1.05) as u64;
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action: set amplification
        let response = env.execute_contract(
            Addr::unchecked(Addr::unchecked(FACTORY_OWNER)),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        ).unwrap();



        // Check the response event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-set-amplification");

        assert_eq!(
            event.attributes[1],
            Attribute::new("target_timestamp", target_timestamp.to_string())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("target_amplification", new_amplification.to_string())
        );


        // Check that the target amplification is set
        let queried_target_amplification = env.get_app().wrap().query_wasm_smart::<TargetAmplificationResponse>(
            vault.clone(),
            &QueryMsg::TargetAmplification {}
        ).unwrap().target_amplification;

        assert_eq!(
            queried_target_amplification,
            Uint64::new(new_amplification)
        );

        // Check that the update target timestamp is set
        let queried_target_timestamp = env.get_app().wrap().query_wasm_smart::<AmplificationUpdateFinishTimestampResponse>(
            vault.clone(),
            &QueryMsg::AmplificationUpdateFinishTimestamp {}
        ).unwrap().timestamp;

        assert_eq!(
            queried_target_timestamp.u64(),
            target_timestamp
        );

    }


    #[test]
    fn test_set_amplification_over_max() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 900000000000000000u64;
        let vault = set_mock_vault(
            &mut env,
            initial_amplification.into()
        );

        // Define the new amplification and the target time
        let max_amplification: u64 = 1000000000000000000u64 - 1u64;
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action 1: set max amplification works 
        env.execute_contract(
            Addr::unchecked(Addr::unchecked(FACTORY_OWNER)),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: max_amplification.into()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // ! Make sure the transaction succeeds



        // Tested action 2: set over max amplification fails 
        let response_result = env.execute_contract(
            Addr::unchecked(Addr::unchecked(FACTORY_OWNER)),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: Uint64::new(max_amplification) + Uint64::one()  // Increase 'max' amplification by 1
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAmplification {}
        ));

    }


    #[test]
    fn test_set_too_large_amplification_change() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 50000000000000000u64;
        let vault = set_mock_vault(
            &mut env,
            initial_amplification.into()
        );

        // Define the new amplification and the target time
        let max_adjustment_factor = 2.;

        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action 1: set amplification max increase works
        let new_amplification = (
            (initial_amplification as f64) * max_adjustment_factor
        ) as u64;

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // Make sure the transaction succeeds



        // Tested action 2: set amplification too large fails
        let new_amplification = (
            (initial_amplification as f64) * max_adjustment_factor
        ) as u64 + 1u64;    // ! Set amplification to one more than allowed

        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAmplification {}
        ));



        // Tested action 3: set amplification max decrease works
        let new_amplification = (
            (initial_amplification as f64) / max_adjustment_factor
        ) as u64;

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        ).unwrap(); // Make sure the transaction succeeds



        // Tested action 4: set amplification too small fails
        let new_amplification = (
            (initial_amplification as f64) / max_adjustment_factor
        ) as u64 - 1u64;    // ! Set amplification to one more than allowed

        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::InvalidAmplification {}
        ))

    }


    #[test]
    fn test_set_amplification_invalid_target_timestamp() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 900000000000000000u64;
        let vault = set_mock_vault(
            &mut env,
            initial_amplification.into()
        );

        // Define the new amplification
        let new_amplification: u64 = ((initial_amplification as f64) * 1.05) as u64;



        // Tested action 1: minimum adjustment time works
        let min_adjustment_time_seconds = 7*24*60*60;   // 7 days
        let target_timestamp = env.get_app().block_info().time.seconds() + min_adjustment_time_seconds;
        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
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
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
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
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
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
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
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
    fn test_set_amplification_unauthorized() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 900000000000000000u64;
        let vault = set_mock_vault(
            &mut env,
            initial_amplification.into()
        );

        // Define the new amplification and the target time
        let new_amplification: u64 = ((initial_amplification as f64) * 1.05) as u64;
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action: set amplification with an invalid caller
        let response_result = env.execute_contract(
            Addr::unchecked("not-factory-owner"),   // ! Not the factory owner
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
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
    fn test_set_amplification_cross_chain_vault() {
        
        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 900000000000000000u64;

        let vault_code_id = amplified_vault_contract_storage(env.get_app());

        let test_tokens =  env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();

        let vault = mock_factory_deploy_vault(
            &mut env,
            test_tokens,
            TEST_VAULT_BALANCES.to_vec(),
            TEST_VAULT_WEIGHTS.to_vec(),
            initial_amplification.into(),
            vault_code_id,
            Some(Addr::unchecked(CHAIN_INTERFACE)), // ! Setup the vault WITH a chain interface
            None
        );

        // Define the new amplification and the target time
        let new_amplification: u64 = ((initial_amplification as f64) * 1.05) as u64;
        let target_timestamp = env.get_app().block_info().time.seconds() + 30*24*60*60;   // 30 days



        // Tested action: set amplification with an invalid caller
        let response_result = env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        );



        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::Error(reason)
                if reason == "Amplification adjustment is disabled for cross-chain vaults.".to_string()
        ))

    }


    #[test]
    fn test_amplification_update_calculation() {

        // Run the same test for amplification decrease and increase
        let change_factors = vec![0.95, 1.05];

        change_factors.iter().for_each(|change_factor| {

            let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

            // Instantiate and initialize vault
            let initial_amplification = 900000000000000000u64;
            let vault = set_mock_vault(
                &mut env,
                initial_amplification.into()
            );
    
            // Define the new amplification and the target time
            let new_amplification: u64 = ((initial_amplification as f64) * change_factor) as u64;
    
            let update_duration = 30*24*60*60;   // 30 days
            let start_timestamp = env.get_app().block_info().time.seconds();
            let target_timestamp = start_timestamp + update_duration;
    
            env.execute_contract(
                Addr::unchecked(FACTORY_OWNER),
                vault.clone(),
                &AmplifiedExecuteMsg::Custom(
                    AmplifiedExecuteExtension::SetAmplification {
                        target_timestamp: target_timestamp.into(),
                        target_amplification: new_amplification.into()
                    }
                ),
                vec![],
                vec![]
            ).unwrap();
    
    
    
            // Test action: Trigger update at different stages of the update and verify the amplification value
            let update_checks = vec![
                0.,
                0.25,
                0.25,   // ! Trigger an update twice on the same block
                0.80,
                1.,
                1.1     // ! Make sure the amplification update stops after 100%
            ];
    
            update_checks
                .iter()
                .for_each(|update_progress| {
                    
                    // Execute a local swap to trigger the amplification recomputation
                    trigger_amplification_update(
                        &mut env,
                        vault.clone(),
                        Timestamp::from_seconds(
                            start_timestamp
                            + (update_duration as f64 * update_progress) as u64
                        )
                    );
    
                    // Verify that the amplification is set correctly
                    let queried_current_amplification = env.get_app().wrap().query_wasm_smart::<AmplificationResponse>(
                        vault.clone(),
                        &QueryMsg::Amplification {}
                    ).unwrap().amplification.u64() as f64;
    
                    let initial_amplification = initial_amplification as f64;
                    let target_amplification = new_amplification as f64;
                    let expected_current_amplification = initial_amplification + (target_amplification - initial_amplification) * update_progress.min(1.);
    
                    assert!(queried_current_amplification <= expected_current_amplification * 1.01);
                    assert!(queried_current_amplification >= expected_current_amplification * 0.99);
                });

        });

    }


    #[test]
    fn test_amplification_update_finish() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let initial_amplification = 900000000000000000u64;
        let vault = set_mock_vault(
            &mut env,
            initial_amplification.into()
        );

        // Define the new amplification and the target time
        let new_amplification: u64 = ((initial_amplification as f64) * 1.05) as u64;

        let update_duration = 30*24*60*60;   // 30 days
        let target_timestamp = env.get_app().block_info().time.seconds() + update_duration;

        env.execute_contract(
            Addr::unchecked(FACTORY_OWNER),
            vault.clone(),
            &AmplifiedExecuteMsg::Custom(
                AmplifiedExecuteExtension::SetAmplification {
                    target_timestamp: target_timestamp.into(),
                    target_amplification: new_amplification.into()
                }
            ),
            vec![],
            vec![]
        ).unwrap();



        // Test action: Trigger the 'update finish' logic of the amplification update

        // Execute a local swap to trigger an amplification recomputation
        trigger_amplification_update(
            &mut env,
            vault.clone(),
            Timestamp::from_seconds(
                target_timestamp        // ! Set the block time to the 'amplification update' finish timestamp
            )
        );
        
        // Verify that the amplification is set correctly
        let queried_current_amplification = env.get_app().wrap().query_wasm_smart::<AmplificationResponse>(
            vault.clone(),
            &QueryMsg::Amplification {}
        ).unwrap().amplification.u64();

        assert!(queried_current_amplification == new_amplification);

        // Check that the time variables have been cleared
        let queried_target_timestamp = env.get_app().wrap().query_wasm_smart::<AmplificationUpdateFinishTimestampResponse>(
            vault.clone(),
            &QueryMsg::AmplificationUpdateFinishTimestamp {}
        ).unwrap().timestamp;

        assert_eq!(
            queried_target_timestamp.u64(),
            0u64
        );
    }


}