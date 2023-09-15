mod test_reentry_lock {
    use cosmwasm_std::{Uint128, Addr};
    use test_helpers::definitions::SETUP_MASTER;
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;

    use crate::{commands::{CommandOrder, CommandMsg}, tests::{helpers::mock_instantiate_router, malicious_vault::mock_instantiate_malicious_vault}, executors::types::CoinAmount, msg::ExecuteMsg};




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
}