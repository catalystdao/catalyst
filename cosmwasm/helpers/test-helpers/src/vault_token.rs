use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, CosmosMsg, BankMsg, Coin};
use cw20_base::state::TokenInfo;
use cw_multi_test::Executor;
use std::fmt::Debug;

use crate::{env::{env_cw20_asset::{Cw20AssetApp, TestCw20AssetEnv}, env_native_asset::{NativeAssetApp, TestNativeAssetEnv}}, token::{query_token_info, query_token_balance, transfer_tokens}};


pub struct VaultTokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8
}

#[cw_serde]
enum TokenInfoQuery {
    TokenInfo {}
}


pub trait CustomTestVaultToken<AppC, TestEnvC>: Clone + Debug + PartialEq {

    fn load(vault: String, denom: String) -> Self;

    fn total_supply(&self, app: &mut AppC) -> Uint128;

    fn query_balance(&self, app: &mut AppC, account: impl Into<String>) -> Uint128;

    fn transfer(&self, app: &mut AppC, amount: Uint128, account: Addr, recipient: impl Into<String>);

    fn query_token_info(&self, app: &mut AppC) -> VaultTokenInfo;

}



#[derive(Debug, Clone, PartialEq)]
pub struct TestNativeVaultToken(String);

impl CustomTestVaultToken<NativeAssetApp, TestNativeAssetEnv> for TestNativeVaultToken {

    fn load(vault: String, denom: String) -> Self {
        TestNativeVaultToken(format!("factory/{}/{}", vault, denom))
    }

    fn total_supply(&self, app: &mut NativeAssetApp) -> Uint128 {
        app.wrap()
            .query_supply(self.0.clone())
            .unwrap()
            .amount
    }

    fn query_balance(&self, app: &mut NativeAssetApp, account: impl Into<String>) -> Uint128 {
        app.wrap()
            .query_balance(
                account,
                self.0.clone()
            )
            .unwrap()
            .amount
    }

    fn transfer(&self, app: &mut NativeAssetApp, amount: Uint128, account: Addr, recipient: impl Into<String>) {

        if amount.is_zero() {
            return;
        }

        app.execute(
            account,
            CosmosMsg::Bank(
                BankMsg::Send {
                    to_address: recipient.into(),
                    amount: vec![Coin::new(amount.u128(), self.0.clone())]
                }
            )
        ).unwrap();
    }

    fn query_token_info(&self, app: &mut NativeAssetApp) -> VaultTokenInfo {
        todo!()
    }
}




#[derive(Debug, Clone, PartialEq)]
pub struct TestCw20VaultToken(String);

impl CustomTestVaultToken<Cw20AssetApp, TestCw20AssetEnv> for TestCw20VaultToken {

    fn load(vault: String, _denom: String) -> Self {
        TestCw20VaultToken(vault)
    }

    fn total_supply(&self, app: &mut Cw20AssetApp) -> Uint128 {
        query_token_info(
            app,
            Addr::unchecked(self.0.clone())
        ).total_supply
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
            recipient.into()
        );
    }

    fn query_token_info(&self, app: &mut Cw20AssetApp) -> VaultTokenInfo {
        
        let token_info: TokenInfo = app
            .wrap()
            .query_wasm_smart::<TokenInfo>(self.0.clone(), &TokenInfoQuery::TokenInfo {})
            .unwrap();

        VaultTokenInfo {
            name: token_info.name.clone(),
            symbol: token_info.symbol.clone(),
            decimals: token_info.decimals
        }
    }
    
}
