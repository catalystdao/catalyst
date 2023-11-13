mod test_volatile_send_liquidity {
    use cosmwasm_std::{Uint128, Addr, Binary, Attribute, coins, coin};
    use catalyst_types::{U256, u256};
    use catalyst_vault_common::{ContractError, msg::{TotalEscrowedLiquidityResponse, LiquidityEscrowResponse}, state::{INITIAL_MINT_AMOUNT, compute_send_liquidity_hash}, bindings::Asset};
    use test_helpers::{math::{uint128_to_f64, f64_to_uint128, u256_to_f64}, misc::{encode_payload_address, get_response_attribute}, definitions::{SETUP_MASTER, CHANNEL_ID, SWAPPER_A, SWAPPER_B, SWAPPER_C, VAULT_TOKEN_DENOM}, contract::{mock_instantiate_interface, mock_factory_deploy_vault, mock_set_vault_connection}, env::CustomTestEnv, vault_token::CustomTestVaultToken, asset::CustomTestAsset};

    use crate::tests::{TestEnv, TestVaultToken, helpers::mock_incentive};
    use crate::{msg::VolatileExecuteMsg, tests::{helpers::{compute_expected_send_liquidity, volatile_vault_contract_storage}, parameters::{TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION, TEST_VAULT_ASSET_COUNT}}};



    #[test]
    fn test_send_liquidity_calculation() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        vault_token.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        // ! Include incentive payment of the same denom as one of the vault's assets to make sure
        // ! it does not affect the send liquidity calculation.
        let additional_coins;

        #[cfg(feature="asset_native")]
        {
            let incentive_asset = vault_assets[0].clone();
            let incentive_amount = vault_initial_balances[0].u128()/100u128;
            incentive_asset.transfer(
                env.get_app(),
                incentive_amount.into(),
                Addr::unchecked(SETUP_MASTER),
                SWAPPER_A.to_string()
            );
            additional_coins = coins(incentive_amount, incentive_asset.denom);
        }
        
        #[cfg(feature="asset_cw20")]
        {
            additional_coins = vec![];
        }
        // For cw20 assets, the incentive payment cannot be a vault asset (incentive is always a coin)


        // Tested action: send liquidity
        let response = env.execute_contract_with_additional_coins(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![],
            vec![],
            additional_coins
        ).unwrap();



        // Verify the swap return
        let expected_return = compute_expected_send_liquidity(
            swap_amount,
            vault_weights.clone(),
            INITIAL_MINT_AMOUNT,
        );

        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        
        assert!(u256_to_f64(observed_return) / 1e18 <= expected_return.u * 1.000001);
        assert!(u256_to_f64(observed_return) / 1e18 >= expected_return.u * 0.999999);


        // Verify the vault tokens have been burnt
        let swapper_vault_tokens_balance = vault_token.query_balance(env.get_app(), SWAPPER_A.to_string());
        assert_eq!(
            swapper_vault_tokens_balance,
            Uint128::zero()
        );
    
        // Verify the vault total vault tokens supply
        let vault_token_supply = vault_token.total_supply(env.get_app());
        assert_eq!(
            vault_token_supply,
            INITIAL_MINT_AMOUNT - swap_amount
        );

        // Verify the vault tokens are escrowed
        let queried_escrowed_total = env.get_app()
            .wrap()
            .query_wasm_smart::<TotalEscrowedLiquidityResponse>(vault.clone(), &crate::msg::QueryMsg::TotalEscrowedLiquidity {  })
            .unwrap()
            .amount;

        assert!(queried_escrowed_total == swap_amount);
    
        // Verify the fallback account/escrow is set
        let expected_liquidity_swap_hash = compute_send_liquidity_hash(
            to_account.as_ref(),
            observed_return,
            swap_amount,
            env.get_app().block_info().height as u32
        );

        let queried_fallback_account = env.get_app()
            .wrap()
            .query_wasm_smart::<LiquidityEscrowResponse>(vault.clone(), &crate::msg::QueryMsg::LiquidityEscrow { hash: Binary(expected_liquidity_swap_hash) })
            .unwrap()
            .fallback_account;

        assert_eq!(
            queried_fallback_account,
            Some(Addr::unchecked(SWAPPER_C))
        );
        

        // Verify interface contract gets invoked
        let invoked_interface = get_response_attribute::<String>(response.events[response.events.len()-1].clone(), "_contract_address").unwrap();
        assert_eq!(
            Addr::unchecked(invoked_interface),
            interface
        );

    }


    #[test]
    fn test_send_liquidity_event() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());
        let min_vault_tokens = u256!("123456789");  // Some random value
        let min_reference_asset = u256!("987654321");  // Some random value

        // Fund swapper with tokens
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        vault_token.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send liquidity
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens,
                min_reference_asset,
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![],
            vec![]
        ).unwrap();



        // Check the event
        let event = response.events[1].clone();

        assert_eq!(event.ty, "wasm-send-liquidity");

        assert_eq!(
            event.attributes[1],
            Attribute::new("channel_id", CHANNEL_ID.to_base64())
        );
        assert_eq!(
            event.attributes[2],
            Attribute::new("to_vault", target_vault.to_base64())
        );
        assert_eq!(
            event.attributes[3],
            Attribute::new("to_account", to_account.to_string())
        );
        assert_eq!(
            event.attributes[4],
            Attribute::new("from_amount", swap_amount)
        );
        assert_eq!(
            event.attributes[5],
            Attribute::new("min_vault_tokens", min_vault_tokens)
        );
        assert_eq!(
            event.attributes[6],
            Attribute::new("min_reference_asset", min_reference_asset)
        );

        //NOTE: 'units' is indirectly checked on `test_send_liquidity_calculation`

    }


    #[test]
    fn test_send_liquidity_zero_amount() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let swap_amount = Uint128::zero();

        // Fund swapper with tokens
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        vault_token.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send liquidity
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify that 0 units are sent
        let observed_return = get_response_attribute::<U256>(response.events[1].clone(), "units").unwrap();
        assert_eq!(
            observed_return,
            U256::zero()
        )

    }


    #[test]
    fn test_send_liquidity_not_connected_vault() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        // ! Do not set the connection with the target vault

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();

        // Fund swapper with tokens
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        vault_token.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );



        // Tested action: send liquidity
        let response_result = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID,
                to_vault: target_vault.clone(),
                to_account: encode_payload_address(SWAPPER_B.as_bytes()),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![],
            vec![]
        );
    


        // Make sure the transaction fails
        assert!(matches!(
            response_result.err().unwrap().downcast().unwrap(),
            ContractError::VaultNotConnected { channel_id: err_channel_id, vault: err_vault }
                if err_channel_id == CHANNEL_ID && err_vault == target_vault
        ));

    }
    

    #[test]
    fn test_send_liquidity_calldata() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        vault_token.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        // Define the calldata
        let target_account = encode_payload_address("CALLDATA_ADDRESS".as_bytes());
        let target_data = vec![0x43, 0x41, 0x54, 0x41, 0x4C, 0x59, 0x53, 0x54];
        let calldata = Binary([target_account.0, target_data].concat());



        // Tested action: send liquidity calldata
        let response = env.execute_contract(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: calldata.clone(),
                incentive: mock_incentive()
            },
            vec![],
            vec![]
        ).unwrap();



        // Verify the swap return
        let payload_calldata = Binary::from_base64(
            &get_response_attribute::<String>(
                response.events[response.events.len()-1].clone(),
                "calldata"
            ).unwrap()
        ).unwrap();

        assert_eq!(
            payload_calldata,
            calldata
        );

    }

    
    #[test]
    fn test_send_liquidity_incentive_relay() {

        let mut env = TestEnv::initialize(SETUP_MASTER.to_string());

        let incentive_coin_denom = "incentive-coin".to_string();
        env.initialize_coin(
            incentive_coin_denom.clone(),
            Uint128::new(1000000000),
            SWAPPER_A.to_string()
        );

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = volatile_vault_contract_storage(env.get_app());
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

        // Set target mock vault
        let target_vault = encode_payload_address(b"target_vault");
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID,
            target_vault.clone(),
            true
        );

        // Define send liquidity configuration
        let send_percentage = 0.15;
        let swap_amount = f64_to_uint128(uint128_to_f64(INITIAL_MINT_AMOUNT) * send_percentage).unwrap();
        let to_account = encode_payload_address(SWAPPER_B.as_bytes());

        // Fund swapper with tokens
        let vault_token = TestVaultToken::load(vault.to_string(), VAULT_TOKEN_DENOM.to_string());
        vault_token.transfer(
            env.get_app(),
            swap_amount,
            Addr::unchecked(SETUP_MASTER),
            SWAPPER_A.to_string()
        );

        let incentive_payment = coin(101u128, incentive_coin_denom.clone());



        // Tested action: send liquidity
        env.execute_contract_with_additional_coins(
            Addr::unchecked(SWAPPER_A),
            vault.clone(),
            &VolatileExecuteMsg::SendLiquidity {
                channel_id: CHANNEL_ID,
                to_vault: target_vault,
                to_account: to_account.clone(),
                amount: swap_amount,
                min_vault_tokens: U256::zero(),
                min_reference_asset: U256::zero(),
                fallback_account: SWAPPER_C.to_string(),
                calldata: Binary(vec![]),
                incentive: mock_incentive()
            },
            vec![],
            vec![],
            vec![incentive_payment.clone()]
        ).unwrap();



        // Verify the incentive payment has been relayed to the interface contract
        let queried_vault_balance = env.get_app().wrap().query_balance(
            vault,
            incentive_coin_denom.clone()
        ).unwrap();

        assert_eq!(
            queried_vault_balance.amount,
            Uint128::zero()
        );

        let queried_interface_balance = env.get_app().wrap().query_balance(
            interface,
            incentive_coin_denom
        ).unwrap();

        assert_eq!(
            queried_interface_balance.amount,
            incentive_payment.amount
        );

    }

}