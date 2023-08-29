use anyhow::{Result as AnyResult, bail};
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Uint128, Coin, Addr};
use cw_multi_test::{App, Executor, AppResponse};
use vault_assets::asset::asset_native::NativeAsset;

use crate::asset::TestNativeAsset;

use super::CustomTestEnv;

pub struct TestNativeAssetEnv(App, Vec<TestNativeAsset>);

impl CustomTestEnv<NativeAsset, TestNativeAsset> for TestNativeAssetEnv {

    fn initialize(gov: String) -> Self {

        //TODO make the following function arguments
        let assets: Vec<TestNativeAsset> = vec![
            TestNativeAsset { denom: "asset1".to_string(), alias: "a1".to_string() },
            TestNativeAsset { denom: "asset2".to_string(), alias: "a2".to_string() },
            TestNativeAsset { denom: "asset3".to_string(), alias: "a3".to_string() },
            TestNativeAsset { denom: "asset4".to_string(), alias: "a4".to_string() },
            TestNativeAsset { denom: "asset5".to_string(), alias: "a5".to_string() }
        ];

        let asset_balances: Vec<Uint128> = vec![
            Uint128::from(100000000000000000000000000u128),
            Uint128::from(100000000000000000000000000u128),
            Uint128::from(100000000000000000000000000u128),
            Uint128::from(100000000000000000000000000u128),
            Uint128::from(100000000000000000000000000u128)
        ];

        let coins = assets.iter()
            .zip(asset_balances)
            .map(|(asset, balance)| {
                Coin::new(balance.u128(), asset.denom.to_string())
            })
            .collect();

        let app = App::new(|router, _, storage| {
            router.bank.init_balance(storage, &Addr::unchecked(gov), coins).unwrap()
        });

        TestNativeAssetEnv(app, assets)
    }

    fn get_app(&mut self) -> &mut App {
        &mut self.0
    }

    fn get_assets(&self) -> Vec<TestNativeAsset> {
        self.1.to_vec()
    }

    fn execute_contract<U: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &U,
        send_assets: Vec<TestNativeAsset>,
        send_amounts: Vec<Uint128>
    ) -> AnyResult<AppResponse> {

        if send_assets.len() != send_amounts.len() {
            bail!("Assets and amounts len mismatch.".to_string());
        }

        // Get funds
        let funds = send_assets.iter()
            .zip(send_amounts)
            .filter(|(_, amount)| !amount.is_zero())    // Bank module does not allow zero-valued amounts
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.clone())
            })
            .collect::<Vec<Coin>>();

        // Execute contract
        self.get_app().execute_contract(
            sender,
            contract_addr,
            msg,
            funds.as_ref()
        )
    }

}
