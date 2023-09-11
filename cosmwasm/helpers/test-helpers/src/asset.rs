
use std::fmt::Debug;
use cosmwasm_std::{Uint128, Addr, Coin, CosmosMsg};
use cw_multi_test::Executor;
use vault_assets::asset::{asset_cw20::Cw20Asset, asset_native::NativeAsset};
use crate::{token::{query_token_balance, transfer_tokens}, env::{env_cw20_asset::Cw20AssetApp, env_native_asset::NativeAssetApp}};


/// Interface for mock test assets.
pub trait CustomTestAsset<CustomApp>: Clone + Debug + PartialEq {

    fn get_asset_ref(&self) -> String;

    fn query_balance(&self, app: &mut CustomApp, account: impl Into<String>) -> Uint128;

    fn transfer(&self, app: &mut CustomApp, amount: Uint128, account: Addr, recipient: impl Into<String>);

}


#[derive(Debug, Clone, PartialEq)]
pub struct TestNativeAsset {
    pub denom: String,
    pub alias: String
}

impl CustomTestAsset<NativeAssetApp> for TestNativeAsset {

    fn get_asset_ref(&self) -> String {
        self.alias.clone()
    }

    fn query_balance(&self, app: &mut NativeAssetApp, account: impl Into<String>) -> Uint128 {

        app.wrap().query_balance(
            account.into(),
            self.denom.clone()
        ).unwrap().amount

    }

    fn transfer(&self, app: &mut NativeAssetApp, amount: Uint128, account: Addr, recipient: impl Into<String>) {

        if amount.is_zero() {
            return;
        }

        app.execute(
            account,
            CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient.into(),
                amount: vec![Coin::new(amount.u128(), self.denom.to_string())]
            })
        ).unwrap();

    }

}

impl From<TestNativeAsset> for NativeAsset {
    fn from(value: TestNativeAsset) -> Self {
        Self {
            denom: value.denom,
            alias: value.alias
        }
    }
}

impl From<NativeAsset> for TestNativeAsset {
    fn from(value: NativeAsset) -> Self {
        Self {
            denom: value.denom,
            alias: value.alias
        }
    }
}




#[derive(Debug, Clone, PartialEq)]
pub struct TestCw20Asset(pub String);

impl CustomTestAsset<Cw20AssetApp> for TestCw20Asset {

    fn get_asset_ref(&self) -> String {
        self.0.clone()
    }

    fn query_balance(&self, app: &mut Cw20AssetApp, account: impl Into<String>) -> Uint128 {

        query_token_balance(
            app,
            Addr::unchecked(self.0.clone()),
            account.into()
        )

    }

    fn transfer(&self, app: &mut Cw20AssetApp, amount: Uint128, account: Addr, recipient: impl Into<String>) {

        transfer_tokens(
            app,
            amount,
            Addr::unchecked(self.0.clone()),
            account,
            recipient.into(),
        );

    }

}

impl From<TestCw20Asset> for Cw20Asset {
    fn from(value: TestCw20Asset) -> Self {
        Self(value.0)
    }
}

impl From<Cw20Asset> for TestCw20Asset {
    fn from(value: Cw20Asset) -> Self {
        Self(value.0)
    }
}

