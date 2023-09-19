mod test_catalyst_commands {
    use catalyst_types::U256;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{Uint64, Uint128, Addr, coin, Binary, CosmosMsg, BankMsg, Deps, QuerierWrapper, Env, ContractInfo};

    use cw_multi_test::Executor;
    use test_helpers::asset::CustomTestAsset;
    use test_helpers::definitions::{SETUP_MASTER, VAULT_TOKEN_DENOM};
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;
    use test_helpers::misc::encode_payload_address;

    use crate::commands::{execute_command, CommandMsg};
    use crate::executors::catalyst::get_vault_token_amount;
    use crate::executors::types::{CoinAmount, Amount};
    use crate::tests::helpers::{MockVault, ROUTER, run_command_result, fund_account};




    #[test]
    fn test_local_swap_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        let from_asset = mock_vault_config.vault_assets[0].clone();
        let to_asset = mock_vault_config.vault_assets[1].clone();
        let swap_amount = coin(100u128, from_asset.denom.clone());

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![swap_amount.clone()]
        );



        // Tested action
        let command_result = execute_command(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            CommandMsg::LocalSwap {
                vault: mock_vault_config.vault.to_string(), 
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_ref: to_asset.get_asset_ref(), 
                amount: CoinAmount::Coin(swap_amount.clone()),
                min_out: Uint128::zero(),
            }
        ).unwrap();



        // Verify the result message works
        let response = run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result,
        );

        // Verify a 'local-swap' event is emitted
        assert_eq!(
            response.events[1].ty,
            "wasm-local-swap".to_string()
        );
    }


    #[test]
    fn test_send_asset_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        let from_asset = mock_vault_config.vault_assets[0].clone();
        let to_asset_index = 1;
        let swap_amount = coin(100u128, from_asset.denom.clone());

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![swap_amount.clone()]
        );



        // Tested action
        let command_result = execute_command(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            CommandMsg::SendAsset {
                vault: mock_vault_config.vault.to_string(),
                channel_id: mock_vault_config.channel_id.clone(),
                to_vault: mock_vault_config.target_vault.clone(),
                to_account: encode_payload_address(b"to-account"),
                from_asset_ref: from_asset.get_asset_ref(),
                to_asset_index,
                amount: CoinAmount::Coin(swap_amount.clone()),
                min_out: U256::zero(),
                fallback_account: "fallback-account".to_string(),
                calldata: Binary(vec![])
            }
        ).unwrap();



        // Verify the result message works
        let response = run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result,
        );

        // Verify a 'send-asset' event is emitted
        assert_eq!(
            response.events[1].ty,
            "wasm-send-asset".to_string()
        );
    }


    #[test]
    fn test_send_liquidity_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        let denom = format!("factory/{}/{}", mock_vault_config.vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let swap_amount_coin = coin(100u128, denom);

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![swap_amount_coin.clone()]
        );



        // Tested action
        let command_result = execute_command(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            CommandMsg::SendLiquidity {
                vault: mock_vault_config.vault.to_string(),
                channel_id: mock_vault_config.channel_id.clone(),
                to_vault: mock_vault_config.target_vault.clone(),
                to_account: encode_payload_address(b"to-account"),
                amount: Amount::Amount(swap_amount_coin.amount),
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: "fallback-account".to_string(),
                calldata: Binary(vec![])
            }
        ).unwrap();



        // Verify the result message works
        let response = run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify a 'send-liquidity' event is emitted
        assert_eq!(
            response.events[1].ty,
            "wasm-send-liquidity".to_string()
        );
    }


    #[test]
    fn test_withdraw_all_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        let denom = format!("factory/{}/{}", mock_vault_config.vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let withdraw_amount_coin = coin(100u128, denom);

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![withdraw_amount_coin.clone()]
        );



        // Tested action
        let command_result = execute_command(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            CommandMsg::WithdrawAll {
                vault: mock_vault_config.vault.to_string(),
                amount: Amount::Amount(withdraw_amount_coin.amount),
                min_out: vec![Uint128::zero(), Uint128::zero()]
            }
        ).unwrap();



        // Verify the result message works
        let response = run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify a 'withdraw' event is emitted
        assert_eq!(
            response.events[1].ty,
            "wasm-withdraw".to_string()
        );
    }


    #[test]
    fn test_withdraw_mixed_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        let denom = format!("factory/{}/{}", mock_vault_config.vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        let withdraw_amount_coin = coin(100u128, denom);

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![withdraw_amount_coin.clone()]
        );



        // Tested action
        let command_result = execute_command(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            CommandMsg::WithdrawMixed {
                vault: mock_vault_config.vault.to_string(),
                amount: Amount::Amount(withdraw_amount_coin.amount),
                withdraw_ratio: vec![Uint64::new(1000000000000000000), Uint64::zero()],
                min_out: vec![Uint128::zero(), Uint128::zero()]
            }
        ).unwrap();



        // Verify the result message works
        let response = run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify a 'withdraw' event is emitted
        assert_eq!(
            response.events[1].ty,
            "wasm-withdraw".to_string()
        );
    }


    #[test]
    fn test_deposit_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        // Include a zero-valued deposit amount.
        let deposit_amounts: Vec<_> = mock_vault_config.vault_assets.iter()
            .enumerate()
            .map(|(index, asset)| {
                coin(100u128 * index as u128, asset.denom.clone())  // Multiply by 'index' to make the first asset amount 0
            })
            .collect();

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            deposit_amounts.clone()
        );



        // Tested action
        let command_result = execute_command(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            CommandMsg::DepositMixed {
                vault: mock_vault_config.vault.to_string(),
                deposit_amounts: deposit_amounts.iter()
                    .map(|coin| CoinAmount::Coin(coin.clone()))
                    .collect(),
                min_out: Uint128::zero()
            }
        ).unwrap();



        // Verify the result message works
        let response = run_command_result(
            &mut test_env,
            Addr::unchecked(ROUTER),
            command_result
        );

        // Verify a 'deposit' event is emitted
        assert_eq!(
            response.events[1].ty,
            "wasm-deposit".to_string()
        );
    }


    #[test]
    fn test_get_vault_token_amount() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);
        let vault = mock_vault_config.vault;



        // Tested action 1: Value amount
        let value = Uint128::new(909u128);
        let amount = Amount::Amount(value);

        let helper_result = get_vault_token_amount(
            &mock_dependencies().as_ref(),
            &mock_env(),
            vault.to_string(),
            amount
        ).unwrap();

        assert_eq!(
            helper_result,
            value
        );



        // Tested action 2: Router balance
        let router_balance = Uint128::new(543u128);
        let denom = format!("factory/{}/{}", vault.clone(), VAULT_TOKEN_DENOM.to_string());
        let amount = Amount::RouterBalance();

        // Send amount to the router
        test_env.get_app().execute(
            Addr::unchecked(SETUP_MASTER),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: vault.to_string(),
                amount: vec![coin(router_balance.u128(), denom)]
            })
        ).unwrap();

        // NOTE: Cannot use `mock_dependencies`, as `get_vault_token_amount` performs a query to
        // the vault contract to get the router's vault token balance.
        let block_info = test_env.get_app().block_info();
        let helper_result = test_env.get_app().read_module(
            |router, api, storage| {

                let router_querier = router.querier(api, storage, &block_info);
                let deps = Deps {
                    storage,
                    api,
                    querier: QuerierWrapper::new(&router_querier),
                };
                let env = Env {
                    block: block_info.clone(),
                    transaction: None,
                    contract: ContractInfo { address: vault.clone() },
                };

                get_vault_token_amount(
                    &deps,
                    &env,
                    vault.to_string(),
                    amount
                ).unwrap()
            }
        );

        assert_eq!(
            helper_result,
            router_balance
        );
    }
}
