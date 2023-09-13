use catalyst_vault_common::{state::DECIMALS, msg::{TotalSupplyResponse, BalanceResponse}};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, CosmosMsg, BankMsg, Coin};
use cw20_base::state::TokenInfo;
use cw_multi_test::Executor;
use token_bindings::Metadata;
use std::fmt::Debug;

use crate::{env::{env_cw20_asset::Cw20AssetApp, env_native_asset::{NativeAssetApp, NativeAssetCustomHandler}}, token::transfer_tokens};


pub struct VaultTokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8
}

#[cw_serde]
enum VaultQueryMsg {
    TotalSupply {},
    Balance { address: String },
    TokenInfo {},
    VaultTokenDenom {}
}

#[cw_serde]
struct VaultTokenDenomResponse {
    denom: String
}


pub trait CustomTestVaultToken<AppC>: Clone + Debug + PartialEq {

    fn load(vault: String, denom: String) -> Self;

    fn total_supply(&self, app: &mut AppC) -> Uint128;

    fn query_balance(&self, app: &mut AppC, account: impl Into<String>) -> Uint128;

    fn transfer(&self, app: &mut AppC, amount: Uint128, account: Addr, recipient: impl Into<String>);

    fn query_token_info(&self, app: &mut AppC) -> VaultTokenInfo;

}



#[derive(Debug, Clone, PartialEq)]
pub struct TestNativeVaultToken{
    vault: String,
    denom: String
}

impl TestNativeVaultToken {

    pub fn denom(&self) -> String {
        self.denom.clone()
    }

    pub fn query_denom(&self, app: &mut NativeAssetApp) -> String {
        app.wrap()
            .query_wasm_smart::<VaultTokenDenomResponse>(
                self.vault.clone(),
                &VaultQueryMsg::VaultTokenDenom {}
            )
            .unwrap()
            .denom
    }
}

impl CustomTestVaultToken<NativeAssetApp> for TestNativeVaultToken {

    fn load(vault: String, denom: String) -> Self {
        TestNativeVaultToken {
            vault: vault.clone(),
            denom: format!("factory/{}/{}", vault, denom)
        }
    }

    fn total_supply(&self, app: &mut NativeAssetApp) -> Uint128 {
        app.wrap()
            .query_wasm_smart::<TotalSupplyResponse>(
                self.vault.clone(),
                &VaultQueryMsg::TotalSupply {}
            )
            .unwrap()
            .total_supply
    }

    fn query_balance(&self, app: &mut NativeAssetApp, account: impl Into<String>) -> Uint128 {
        app.wrap()
            .query_wasm_smart::<BalanceResponse>(
                self.vault.clone(),
                &VaultQueryMsg::Balance { address: account.into() }
            )
            .unwrap()
            .balance
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
                    amount: vec![Coin::new(amount.u128(), self.denom.clone())]
                }
            )
        ).unwrap();
    }

    fn query_token_info(&self, app: &mut NativeAssetApp) -> VaultTokenInfo {

        let metadata = app.read_module::<_, Option<Metadata>>(|_, _, storage| {
            NativeAssetCustomHandler::load_denom_metadata(storage, self.denom.clone()).unwrap()
        });

        match metadata {
            Some(metadata) => VaultTokenInfo {
                name: metadata.name.unwrap_or("".to_string()),
                symbol: metadata.symbol.unwrap_or("".to_string()),
                decimals: DECIMALS, // Hardcode decimals, as these are not set on the TokenFactory vault token
            },
            None => panic!("No metadata found for denom {}", self.denom),
        }
    }
}




#[derive(Debug, Clone, PartialEq)]
pub struct TestCw20VaultToken {
    vault: String
}

impl CustomTestVaultToken<Cw20AssetApp> for TestCw20VaultToken {

    fn load(vault: String, _denom: String) -> Self {
        TestCw20VaultToken {
            vault
        }
    }

    fn total_supply(&self, app: &mut Cw20AssetApp) -> Uint128 {
        app.wrap()
            .query_wasm_smart::<TotalSupplyResponse>(
                self.vault.clone(),
                &VaultQueryMsg::TotalSupply {}
            )
            .unwrap()
            .total_supply
    }

    fn query_balance(&self, app: &mut Cw20AssetApp, account: impl Into<String>) -> Uint128 {
        app.wrap()
            .query_wasm_smart::<BalanceResponse>(
                self.vault.clone(),
                &VaultQueryMsg::Balance { address: account.into() }
            )
            .unwrap()
            .balance
    }

    fn transfer(&self, app: &mut Cw20AssetApp, amount: Uint128, account: Addr, recipient: impl Into<String>) {
        transfer_tokens(
            app,
            amount,
            Addr::unchecked(self.vault.clone()),
            account,
            recipient.into()
        );
    }

    fn query_token_info(&self, app: &mut Cw20AssetApp) -> VaultTokenInfo {
        
        let token_info: TokenInfo = app
            .wrap()
            .query_wasm_smart::<TokenInfo>(self.vault.clone(), &VaultQueryMsg::TokenInfo {})
            .unwrap();

        VaultTokenInfo {
            name: token_info.name.clone(),
            symbol: token_info.symbol.clone(),
            decimals: token_info.decimals
        }
    }
    
}
