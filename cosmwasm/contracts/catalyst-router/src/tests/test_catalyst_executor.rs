mod test_catalyst_executor {
    use catalyst_types::U256;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{Uint64, Uint128, to_binary, Addr, coin, Binary};

    use test_helpers::asset::CustomTestAsset;
    use test_helpers::definitions::{SETUP_MASTER, VAULT_TOKEN_DENOM};
    use test_helpers::env::CustomTestEnv;
    use test_helpers::env::env_native_asset::TestNativeAssetEnv;
    use test_helpers::misc::encode_payload_address;

    use crate::executors::catalyst::catalyst_executors::{execute_local_swap, LocalSwapCommand, SendAssetCommand, execute_send_asset, SendLiquidityCommand, execute_send_liquidity, DepositMixedCommand, execute_deposit_mixed, WithdrawMixedCommand, execute_withdraw_mixed, WithdrawAllCommand, execute_withdraw_equal};
    use crate::executors::types::types::{CoinAmount, Amount};
    use crate::tests::helpers::{MockVault, ROUTER, run_command_result, fund_account};




    #[test]
    fn test_local_swap_command() {

        let mut test_env = TestNativeAssetEnv::initialize(SETUP_MASTER.to_string());

        let mock_vault_config = MockVault::new(&mut test_env);

        let from_asset = mock_vault_config.vault_assets[0].clone();
        let to_asset = mock_vault_config.vault_assets[1].clone();
        let swap_amount = coin(100u128, from_asset.denom.clone());

        let local_swap_command = LocalSwapCommand {
            vault: mock_vault_config.vault.to_string(),
            from_asset_ref: from_asset.get_asset_ref(),
            to_asset_ref: to_asset.get_asset_ref(),
            amount: CoinAmount::Coin(swap_amount.clone()),
            min_out: Uint128::zero(),
        };

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![swap_amount]
        );



        // Tested action
        let command_result = execute_local_swap(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            &to_binary(&local_swap_command).unwrap()
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

        let send_asset_command = SendAssetCommand {
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
        };

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![swap_amount]
        );



        // Tested action
        let command_result = execute_send_asset(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            &to_binary(&send_asset_command).unwrap()
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

        let send_liquidity_command = SendLiquidityCommand {
            vault: mock_vault_config.vault.to_string(),
            channel_id: mock_vault_config.channel_id.clone(),
            to_vault: mock_vault_config.target_vault.clone(),
            to_account: encode_payload_address(b"to-account"),
            amount: Amount::Amount(swap_amount_coin.amount),
            min_vault_tokens: U256::zero(),
            min_reference_asset: U256::zero(),
            fallback_account: "fallback-account".to_string(),
            calldata: Binary(vec![])
        };

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![swap_amount_coin]
        );



        // Tested action
        let command_result = execute_send_liquidity(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            &to_binary(&send_liquidity_command).unwrap()
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

        let send_liquidity_command = WithdrawAllCommand {
            vault: mock_vault_config.vault.to_string(),
            amount: Amount::Amount(withdraw_amount_coin.amount),
            min_out: vec![Uint128::zero(), Uint128::zero()]
        };

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![withdraw_amount_coin]
        );



        // Tested action
        let command_result = execute_withdraw_equal(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            &to_binary(&send_liquidity_command).unwrap()
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

        let send_liquidity_command = WithdrawMixedCommand {
            vault: mock_vault_config.vault.to_string(),
            amount: Amount::Amount(withdraw_amount_coin.amount),
            withdraw_ratio: vec![Uint64::new(1000000000000000000), Uint64::zero()],
            min_out: vec![Uint128::zero(), Uint128::zero()]
        };

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            vec![withdraw_amount_coin]
        );



        // Tested action
        let command_result = execute_withdraw_mixed(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            &to_binary(&send_liquidity_command).unwrap()
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

        let deposit_amounts: Vec<_> = mock_vault_config.vault_assets.iter()
            .map(|asset| coin(100u128, asset.denom.clone()))
            .collect();

        let deposit_command = DepositMixedCommand {
            vault: mock_vault_config.vault.to_string(),
            deposit_amounts: deposit_amounts.iter()
                                .map(|coin| CoinAmount::Coin(coin.clone()))
                                .collect(),
            min_out: Uint128::zero(),
        };

        fund_account(
            &mut test_env,
            Addr::unchecked(ROUTER),
            deposit_amounts
        );



        // Tested action
        let command_result = execute_deposit_mixed(
            &mock_dependencies().as_ref(),  // Can use mock_dependencies, as no state is required for this test
            &mock_env(),                    // Can use mock_env, as no state is required for this test
            &to_binary(&deposit_command).unwrap()
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
}
