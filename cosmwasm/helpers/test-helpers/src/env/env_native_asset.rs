use anyhow::{Result as AnyResult, bail};
use cosmwasm_schema::{serde::{Serialize, de::DeserializeOwned}, schemars::JsonSchema};
use cosmwasm_std::{Uint128, Coin, Addr, Empty, Api, Storage, BlockInfo, CustomQuery, Querier, Binary, coins, BankMsg};
use cosmwasm_storage::{prefixed, prefixed_read};
use cw_multi_test::{Executor, AppResponse, Module, CosmosRouter, BasicAppBuilder, BankKeeper, BankSudo};

use catalyst_vault_common::asset::native_asset_vault_modules::NativeAssetCustomMsg;
use cw_storage_plus::Map;
use token_bindings::{TokenMsg, Metadata};

use crate::asset::TestNativeAsset;
use super::{CustomTestEnv, CustomApp};



// Custom handler to handle TokenFactory messages
pub struct NativeAssetCustomHandler {}
pub type NativeAssetApp = CustomApp<NativeAssetCustomHandler, NativeAssetCustomMsg>;

impl NativeAssetCustomHandler {

    const BANK_METADATA_NAMESPACE: &[u8] = b"bank-metadata";
    const BANK_METADATA: Map<'static, String, Metadata> = Map::new("denom-metadata");

    // Extend the 'BankKeeper' functionality with a new storage map that holds denom metadata
    // NOTE: The following code mirrors the logic of `cw_multi_test::bank.rs`

    pub fn save_denom_metadata(
        storage: &mut dyn Storage,
        denom: String,
        metadata: Metadata
    ) -> AnyResult<()> {
        let mut bank_metadata_storage = prefixed(storage, Self::BANK_METADATA_NAMESPACE);
        Self::BANK_METADATA
            .save(&mut bank_metadata_storage, denom, &metadata)
            .map_err(Into::into)
    }

    pub fn load_denom_metadata(
        storage: &dyn Storage,
        denom: String
    ) -> AnyResult<Option<Metadata>> {
        let mut bank_metadata_storage = prefixed_read(storage, Self::BANK_METADATA_NAMESPACE);
        
        Ok(
            Self::BANK_METADATA
                .load(&mut bank_metadata_storage, denom)
                .ok()
        )

    }
}

impl Module for NativeAssetCustomHandler {
    type ExecT = NativeAssetCustomMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC:
            std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {

        match msg {
            NativeAssetCustomMsg::Token(token_msg) => {
                match token_msg {

                    TokenMsg::CreateDenom { subdenom, metadata } => {

                        let denom = format!("factory/{}/{}", sender.to_string(), subdenom);

                        if let Some(metadata) = metadata {
                            Self::save_denom_metadata(storage, denom.clone(), metadata)?;
                        }

                        let bank = BankKeeper::new();
                        bank.init_balance(storage, &sender, coins(0u128, denom))?
                    },

                    TokenMsg::MintTokens {
                        denom,
                        amount,
                        mint_to_address
                    } => {

                        // Check sender
                        // NOTE: The sender should be checked against the token ADMIN instead, but
                        // this is fine for the purposes of the Catalyst vault tests
                        if sender != get_denom_creator(denom.clone()) {
                            panic!("Unable to mint native token: sender does not match token creator.")
                        }

                        let bank = BankKeeper::new();
                        bank.sudo(
                            api,
                            storage,
                            router,
                            block,
                            BankSudo::Mint {
                                to_address: mint_to_address,
                                amount: coins(amount.u128(), denom),
                            }
                        )?;
                        
                    },

                    TokenMsg::BurnTokens {
                        denom,
                        amount,
                        burn_from_address
                    } => {

                        // Check sender
                        // NOTE: The sender should be checked against the token ADMIN instead, but
                        // this is fine for the purposes of the Catalyst vault tests
                        if sender != get_denom_creator(denom.clone()) {
                            panic!("Unable to mint native token: sender does not match token creator.")
                        }

                        let bank = BankKeeper::new();
                        bank.execute(
                            api,
                            storage,
                            router,
                            block,
                            Addr::unchecked(burn_from_address), // The 'Bank' module expects the 'burn' msg to come from the token holder
                            BankMsg::Burn { amount: coins(amount.u128(), denom) }
                        )?;

                    },

                    _ => panic!("Custom test handler unable to process the requested TokenFactory msg")

                }
            },
        };
        

        Ok(AppResponse::default())

    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC:
            std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        bail!("sudo not implemented for NativeAssetCustomHandler")
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        bail!("query not implemented for NativeAssetCustomHandler")
    }
}

pub struct TestNativeAssetEnv(NativeAssetApp, Vec<TestNativeAsset>);

impl CustomTestEnv<NativeAssetApp, TestNativeAsset> for TestNativeAssetEnv {

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

        let app = BasicAppBuilder::<NativeAssetCustomMsg, Empty>::new_custom()
            .with_custom(NativeAssetCustomHandler {})
            .build(|router, _, storage| {
                router.bank.init_balance(storage, &Addr::unchecked(gov), coins).unwrap()
            });

        TestNativeAssetEnv(app, assets)
    }

    fn get_app(&mut self) -> &mut NativeAssetApp {
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


fn get_denom_creator(denom: String) -> String {

    if !denom.starts_with("factory/") {
        panic!("Invalid native token denom (must start with 'factory/')");
    }

    let denom_split = denom[8..].split_once("/");

    match denom_split {
        Some((creator_split, _)) => {
            creator_split.to_string()
        },
        None => {
            denom[8..].to_string()
        },
    }

}
