
use std::fmt::Debug;
use cosmwasm_std::{Uint128, Addr, Coin, CosmosMsg};
use cw_multi_test::{App, Executor};
use vault_assets::asset::{AssetTrait, asset_cw20::Cw20Asset, asset_native::NativeAsset};
use crate::token::{query_token_balance, transfer_tokens};


pub trait CustomTestAsset<T: AssetTrait>: Clone + Debug {

    fn get_asset_ref(&self) -> &str;

    fn query_balance(&self, app: &mut App, account: impl Into<String>) -> Uint128;

    fn transfer(&self, app: &mut App, amount: Uint128, account: Addr, recipient: impl Into<String>);

    fn into_vault_asset(&self) -> T;

}


#[derive(Debug, Clone)]
pub struct TestNativeAsset {
    pub denom: String,
    pub alias: String
}

impl CustomTestAsset<NativeAsset> for TestNativeAsset {

    fn get_asset_ref(&self) -> &str {
        &self.alias
    }

    fn query_balance(&self, app: &mut App, account: impl Into<String>) -> Uint128 {

        app.wrap().query_balance(
            account.into(),
            self.denom.clone()
        ).unwrap().amount

    }

    fn transfer(&self, app: &mut App, amount: Uint128, account: Addr, recipient: impl Into<String>) {

        app.execute(
            account,
            CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient.into(),
                amount: vec![Coin::new(amount.u128(), self.denom.to_string())]
            })
        ).unwrap();

    }

    fn into_vault_asset(&self) -> NativeAsset {
        NativeAsset {
            denom: self.denom.to_string(),
            alias: self.alias.to_string()
        }
    }
}




#[derive(Debug, Clone)]
pub struct TestCw20Asset(pub String);

impl CustomTestAsset<Cw20Asset> for TestCw20Asset {

    fn get_asset_ref(&self) -> &str {
        &self.0
    }

    fn query_balance(&self, app: &mut App, account: impl Into<String>) -> Uint128 {

        query_token_balance(
            app,
            Addr::unchecked(self.0.clone()),
            account.into()
        )

    }

    fn transfer(&self, app: &mut App, amount: Uint128, account: Addr, recipient: impl Into<String>) {

        transfer_tokens(
            app,
            amount,
            Addr::unchecked(self.0.clone()),
            account,
            recipient.into(),
        );

    }

    fn into_vault_asset(&self) -> Cw20Asset {
        Cw20Asset(self.0.to_string())
    }

}

