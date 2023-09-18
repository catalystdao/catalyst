mod test_reentry_lock {
    use cosmwasm_std::{Uint128, Addr, coin};
    use test_helpers::definitions::SETUP_MASTER;
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;

    use crate::{commands::{CommandOrder, CommandMsg}, tests::{helpers::{mock_instantiate_router, RECIPIENT}, malicious_vault::mock_instantiate_malicious_vault}, executors::types::{CoinAmount, Account}, msg::ExecuteMsg};




    #[test]
    fn test_try_reentry() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());
        let malicious_vault = mock_instantiate_malicious_vault(test_env.get_app(), router.clone());

        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::LocalSwap {
                    vault: malicious_vault.to_string(),
                    from_asset_ref: "a".to_string(),
                    to_asset_ref: "b".to_string(),
                    amount: CoinAmount::RouterBalance(assets[0].denom.clone()),
                    min_out: Uint128::zero()
                },
                allow_revert: false
            }
        ];



        // Tested action: swap into malicious vault that will try to reenter the router
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



        // Make sure the transaction fails
        assert!(
            result.is_err()
        );

    }


    /// The router reentry lock can be removed on 2 different places. This test checks for removal
    /// of the lock on the `execute_execute` function (that is, when the router only receives
    /// 'check' commands and doesn't generate any submessage).
    #[test]
    fn test_unlock_on_execute() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());

        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {     // ! Check only
                    denoms: vec![assets[0].denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Address(SETUP_MASTER.to_string())
                },
                allow_revert: false
            }
        ];



        // Tested action
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            vec![],
            vec![]
        ).unwrap();



        // Make sure the router can be invoked again (i.e. the lock has been removed)
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            vec![],
            vec![]
        ).unwrap();

    }


    /// The router reentry lock can be removed on 2 different places. This test checks for removal
    /// of the lock on the `reply` handler (that is, after the router has processed all of the
    /// required submessages.
    #[test]
    fn test_unlock_on_reply() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());

        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::Transfer {     // ! Require CosmosMsg
                    amounts: vec![CoinAmount::Coin(coin(1000u128, assets[0].denom.clone()))],
                    recipient: Account::Address(RECIPIENT.to_string())
                },
                allow_revert: false
            },
        ];



        // Tested action
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            assets.clone(),
            vec![Uint128::new(1000u128)]    // Send assets to the router
        ).unwrap();



        // Make sure the router can be invoked again (i.e. the lock has been removed)
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128)]    // Send assets to the router
        ).unwrap();

    }


    /// As `test_unlock_on_reply`, but for the special case in which the last command is
    /// a 'check' operation ('checks' are executed atomically within the 'reply' handler).
    #[test]
    fn test_unlock_on_reply_with_final_check() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let assets = test_env.get_assets()[..1].to_vec();

        let router = mock_instantiate_router(test_env.get_app());

        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::Transfer {     // ! Require CosmosMsg
                    amounts: vec![CoinAmount::Coin(coin(1000u128, assets[0].denom.clone()))],
                    recipient: Account::Address(RECIPIENT.to_string())
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::BalanceCheck { // ! Final check
                    denoms: vec![assets[0].denom.clone()],
                    minimum_amounts: vec![Uint128::zero()],
                    account: Account::Address(SETUP_MASTER.to_string())
                },
                allow_revert: true
            }
        ];



        // Tested action
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders: command_orders.clone(),
                deadline: None
            },
            assets.clone(),
            vec![Uint128::new(1000u128)]    // Send assets to the router
        ).unwrap();



        // Make sure the router can be invoked again (i.e. the lock has been removed)
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::Execute {
                command_orders,
                deadline: None
            },
            assets,
            vec![Uint128::new(1000u128)]    // Send assets to the router
        ).unwrap();

    }
}
