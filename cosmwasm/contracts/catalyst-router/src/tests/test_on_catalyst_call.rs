mod test_catalyst_on_call {


    use cosmwasm_std::{Uint128, Addr, to_json_binary};
    use test_helpers::definitions::SETUP_MASTER;
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;

    use crate::{tests::helpers::{mock_instantiate_router, RECIPIENT}, commands::{CommandOrder, CommandMsg}, executors::types::Account, msg::{ExecuteMsg, ExecuteParams}};


    #[test]
    fn test_on_catalyst_call() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());
        let asset = test_env.get_assets()[0].clone();

        let router = mock_instantiate_router(test_env.get_app());

        let command_orders = vec![
            CommandOrder {
                command: CommandMsg::BalanceCheck {
                    denoms: vec![asset.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    account: Account::Router
                },
                allow_revert: false
            },
            CommandOrder {
                command: CommandMsg::Sweep {
                    denoms: vec![asset.denom.clone()],
                    minimum_amounts: vec![Uint128::one()],
                    recipient: Account::Address(RECIPIENT.to_string())
                },
                allow_revert: false
            }
        ];



        // Tested action: Simulate an 'onCatalystCall'
        test_env.execute_contract(
            Addr::unchecked(SETUP_MASTER),
            router.clone(),
            &ExecuteMsg::OnCatalystCall {
                purchased_tokens: Uint128::new(0u128),
                data: to_json_binary(&ExecuteParams{
                    command_orders,
                    deadline: None,
                }).unwrap()
            },
            vec![asset],
            vec![Uint128::new(1000u128)]    // Send assets to the router
        ).unwrap();




        // Check the router has performed the last 'sweep' operation
        let router_balances = test_env.get_app()
            .wrap()
            .query_all_balances(router)
            .unwrap()
            .len();

        assert_eq!(
            router_balances,
            0
        );

    }

}