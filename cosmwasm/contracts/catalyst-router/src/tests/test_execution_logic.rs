mod test_execution_logic {

    use cosmwasm_std::{Uint128, Addr, Coin, coin, StdError, Binary};
    use std::iter;

    use test_helpers::definitions::SETUP_MASTER;
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;

    use crate::{commands::{CommandOrder, CommandMsg}, tests::helpers::mock_instantiate_router, executors::types::{Account, CoinAmount}, msg::ExecuteMsg, error::ContractError};


    // Generate a standard 'balance check' and 'sweep' router order
    fn mock_router_command(amount: Coin) -> Vec<CommandOrder> {
        vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {
                    denoms: vec![amount.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![amount.denom],
                    minimum_amounts: vec![Uint128::one()],
                    recipient: Account::Sender
                },
                allow_revert: false
            }
        ]
    }



    // Command Orders General Handling Tests
    // ********************************************************************************************

    #[test]
    fn test_router_commands() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());
        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {
                    denoms: vec![amount.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![amount.denom],
                    minimum_amounts: vec![Uint128::one()],
                    recipient: Account::Sender
                },
                allow_revert: false
            }
        ];



        // Tested action
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128)]    // Send assets to the router
        );



        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

        // Check the router has performed the last 'sweep' operation
        let router_balances = test_env.get_app()
            .wrap()
            .query_all_balances(router.clone())
            .unwrap()
            .len();

        assert_eq!(
            router_balances,
            0
        );

        // Check the router has not created a state
        let router_state_raw = test_env.get_app().wrap().query_wasm_raw(
            router,
            "router-state".as_bytes()
        ).unwrap();

        assert!(router_state_raw.is_none());

    }


    #[test]
    fn test_router_commands_only_check() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {         // ! Include only a 'check' command (no submessages generated)
                    denoms: vec![assets[0].denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            }
        ];



        // Tested action 1: Check fails
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            assets.clone(),
            vec![]  // ! Do not send funds to the router
        );

        // Make sure the transaction passes
        assert!(
            result.is_err()
        );



        // Tested action 2: Check passes
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128)]    // ! Send funds to the router
        );

        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

        // Check the router has not created a state
        let router_state_raw = test_env.get_app().wrap().query_wasm_raw(
            router,
            "router-state".as_bytes()
        ).unwrap();

        assert!(router_state_raw.is_none());

    }


    #[test]
    fn test_router_commands_resume_on_reply() {

        // Test that the router resumes messages dispatching on the reply handler correctly.

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());

        // ! Set **2** commands that require submessage execution to force the reply handler to
        // ! resume command dispatching.
        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::Transfer {
                    amounts: vec![CoinAmount::Coin(coin(1u128, assets[0].denom.clone()))],
                    recipient: Account::Sender
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![amount.denom],
                    minimum_amounts: vec![Uint128::one()],
                    recipient: Account::Sender
                },
                allow_revert: false
            }
        ];



        // Tested action
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128)]
        );



        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

        // Check the router has performed the last 'sweep' operation
        let router_balances = test_env.get_app()
            .wrap()
            .query_all_balances(router.clone())
            .unwrap()
            .len();

        assert_eq!(
            router_balances,
            0
        );

        // Check the router has cleared its state
        let router_state_raw = test_env.get_app().wrap().query_wasm_raw(
            router,
            "router-state".as_bytes()
        ).unwrap();

        assert!(router_state_raw.is_none());

    }


    #[test]
    fn test_router_commands_resume_on_reply_with_final_check() {

        // Like `test_router_commands_resume_on_reply`, but having the last command perform
        // a 'check' operation instead of generating a submessage (this runs extra logic on
        // the reply handler).

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());

        // Set **2** commands that require submessage execution to force the reply handler to
        // resume command dispatching.
        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::Transfer {
                    amounts: vec![CoinAmount::Coin(coin(1u128, assets[0].denom.clone()))],  // ! Transfer 1 coin
                    recipient: Account::Sender
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::BalanceCheck {
                    denoms: vec![assets[0].denom.clone()],
                    minimum_amounts: vec![Uint128::one()],  // ! Check the router has 1 coin left
                    account: Account::Router
                },
                allow_revert: false
            }
        ];



        // Tested action 1: Final check fails
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            assets.clone(),
            vec![Uint128::new(1u128)]   // ! Send only 1 coin
        );

        // Make sure the transaction fails
        assert!(
            result.is_err()
        );



        // Tested action 1: Final check passes
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(2u128)]   // ! Send 2 coin
        );

        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

        // Check the router has cleared its state
        let router_state_raw = test_env.get_app().wrap().query_wasm_raw(
            router,
            "router-state".as_bytes()
        ).unwrap();

        assert!(router_state_raw.is_none());

    }


    #[test]
    fn test_router_commands_empty() {

        // NOTE: This test acknowledges that the router execution without commands will behave gracefully.

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let router = mock_instantiate_router(test_env.get_app());



        // Tested action
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders: vec![], // No commands
                deadline: None
            },
            vec![],
            vec![]
        );



        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

    }


    #[test]
    fn test_router_commands_too_many_initial_checks() {

        // The router executes as many consecutive 'CommandOrder's as possible at the start of the
        // commands dispatchment until a submessage is generated. Once a 'CommandOrder' that does
        // require a submessage execution is reached, the leftover commands are saved to storage
        // to be able to resume dispatching once the submessage execution completes. An 'offset' 
        // variable is also stored to track how many 'CommandOrder's have been executed at this 
        // initial stage. This test checks that this 'offset' variable cannot overflow.

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());

        // ! Perform 255 initial balance checks ('offset' is u8)
        // Then perform 2 further commands to force the router to save state to storage.
        let command_orders: Vec<CommandOrder> = iter::repeat(
            CommandOrder {
                command: CommandMsg::BalanceCheck {
                    denoms: vec![amount.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            }
        )
            .take(255)
            .chain(vec![
                CommandOrder {
                    command: CommandMsg::Sweep {
                        denoms: vec![amount.denom],
                        minimum_amounts: vec![Uint128::one()],
                        recipient: Account::Sender
                    },
                    allow_revert: false
                },
                CommandOrder {
                    command: CommandMsg::Sweep {
                        denoms: vec![],
                        minimum_amounts: vec![],
                        recipient: Account::Sender
                    },
                    allow_revert: false
                }
            ]).collect();



        // Tested action
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128), Uint128::new(1000u128)]
        );



        // Make sure the transaction does not pass
        assert!(matches!(
            result.err().unwrap().root_cause().downcast_ref().unwrap(),
            StdError::GenericErr { msg }
                if msg == "Failed to save the router state offset (too many commands)."
        ));

    }



    // Failed Command Orders Tests
    // ********************************************************************************************

    #[test]
    fn test_router_commands_failed_command_submessage() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());

        let mut command_orders = vec![
            CommandOrder {
                command: CommandMsg::Transfer {
                    amounts: vec![CoinAmount::Coin(coin(1u128, assets[0].denom.clone()))],
                    recipient: Account::Sender
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![amount.denom],
                    minimum_amounts: vec![Uint128::zero()],
                    recipient: Account::Sender
                },
                allow_revert: false
            }
        ];



        // Tested action 1: 'allow_revert' set to 'false'
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            vec![],    // ! Do not send funds to the router to cause the first command to fail
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            result.err().unwrap().downcast().unwrap(),
            ContractError::CommandReverted { index, error: _ }
                if index == 0
        ));



        // Tested action 2: 'allow_revert' set to 'true'
        command_orders[0].allow_revert = true;
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            vec![],    // ! Do not send funds to the router to cause the first command to fail
            vec![]
        );

        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

    }


    /// Like `test_router_commands_failed_command_submessage`, but with the **last** message being
    /// the one that fails. This test intends to verify that the 'command index' variable is
    /// correctly computed when resuming dispatching on the `reply` handler.
    #[test]
    fn test_router_commands_failed_command_submessage_last() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());

        let mut command_orders = vec![
            CommandOrder {                                  // ! Perform balance check
                command: CommandMsg::BalanceCheck {
                    denoms: vec![amount.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router,
                },
                allow_revert: false
            },
            CommandOrder {                                  // ! Perform balance check
                command: CommandMsg::BalanceCheck {
                    denoms: vec![amount.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router,
                },
                allow_revert: false
            },
            CommandOrder {                                  // ! Create a CosmosMsg
                command: CommandMsg::Transfer {
                    amounts: vec![CoinAmount::Coin(amount.clone())],
                    recipient: Account::Sender
                },
                allow_revert: false
            },
            CommandOrder {                                  // ! Create a failing CosmosMsg
                command: CommandMsg::Transfer {
                    amounts: vec![CoinAmount::Coin(amount.clone())],
                    recipient: Account::Sender
                },
                allow_revert: false
            },
        ];



        // Tested action 1: 'allow_revert' set to 'false'
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            vec![assets[0].clone()],
            vec![amount.amount]
        );

        // Make sure the transaction fails
        assert!(matches!(
            result.err().unwrap().downcast().unwrap(),
            ContractError::CommandReverted { index, error: _ }
                if index == 3   // ! Make sure that the failing message is the fourth one (index=3).
        ));



        // Tested action 2: 'allow_revert' set to 'true'
        command_orders[3].allow_revert = true;
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            vec![assets[0].clone()],
            vec![amount.amount]
        );

        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

    }


    #[test]
    fn test_router_commands_failed_command_check() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());

        let mut command_orders = vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {     // ! Include a 'check'
                    denoms: vec![assets[0].denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![amount.denom],
                    minimum_amounts: vec![Uint128::zero()],
                    recipient: Account::Sender
                },
                allow_revert: false
            }
        ];



        // Tested action 1: 'allow_revert' set to 'false'
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            vec![],    // ! Do not send funds to the router to cause the first command to fail
            vec![]
        );

        // Make sure the transaction fails
        assert!(matches!(
            result.err().unwrap().downcast().unwrap(),
            ContractError::CommandReverted { index, error: _ }
                if index == 0
        ));



        // Tested action 2: 'allow_revert' set to 'true'
        command_orders[0].allow_revert = true;
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            vec![],    // ! Do not send funds to the router to cause the first command to fail
            vec![]
        );

        // Make sure the transaction ignores the 'allow_revert' flag and fails
        assert!(matches!(
            result.err().unwrap().downcast().unwrap(),
            ContractError::CommandReverted { index, error: _ }
                if index == 0
        ));

    }



    // Deadline Tests
    // ********************************************************************************************

    #[test]
    fn test_deadline_valid() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());
        let command_orders = mock_router_command(amount);
        let current_time = test_env.get_app().block_info().time.seconds();



        // Tested action
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: Some(current_time)    // Set the current time as the deadline
            },
            assets,
            vec![Uint128::new(1000u128), Uint128::new(1000u128)]
        );



        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );

    }


    #[test]
    fn test_deadline_invalid() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());
        let command_orders = mock_router_command(amount);
        let current_time = test_env.get_app().block_info().time.seconds();



        // Tested action
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: Some(current_time - 1)    // Set the deadline as one second in the past
            },
            assets,
            vec![Uint128::new(1000u128), Uint128::new(1000u128)]
        );



        // Make sure the transaction passes
        assert!(matches!(
            result.err().unwrap().downcast().unwrap(),
            ContractError::TransactionDeadlinePassed {}
        ));

    }



    // Cancel Swap Tests
    // ********************************************************************************************

    #[test]
    fn test_cancel_swap() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..2].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let amount = coin(1000u128, assets[0].denom.clone());
        let allow_cancel_id = Binary::from("cancel-id".as_bytes());
        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {
                    denoms: vec![amount.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![amount.denom],
                    minimum_amounts: vec![Uint128::one()],
                    recipient: Account::Sender
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::AllowCancel {
                    authority: SETUP_MASTER.to_string(),
                    identifier: allow_cancel_id.clone()
                },
                allow_revert: false
            }
        ];

        // ! Do not 'cancel' the swap



        // Tested action 1: 'cancel' unset
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            assets.clone(),
            vec![Uint128::new(1000u128), Uint128::new(1000u128)]
        );

        // Make sure the transaction passes
        assert!(
            result.is_ok()
        );



        // Tested action 2: 'cancel' set

        // Set the 'cancel' state
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::CancelSwap {
                identifier: allow_cancel_id.clone(),
                state: None     // ! 'Cancel' the swap
            },
            assets.clone(),
            vec![Uint128::new(1000u128), Uint128::new(1000u128)]
        ).unwrap();

        // Execute commands
        let result = test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router,
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128), Uint128::new(1000u128)]
        );
        
        // Make sure the transaction fails
        assert!(matches!(
            result.err().unwrap().downcast().unwrap(),
            ContractError::CommandReverted { index, error }
                if index == 2 && error == format!(
                    "Swap cancelled (authority {}, identifier {})",
                    SETUP_MASTER,
                    allow_cancel_id.to_base64()
                )
        ));

    }

}