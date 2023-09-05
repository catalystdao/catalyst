use anyhow::{Result as AnyResult, bail};
use catalyst_vault_common::asset::CustomMsg;
use cosmwasm_schema::{serde::{Serialize, de::DeserializeOwned}, schemars::JsonSchema};
use cosmwasm_std::{Uint128, Coin, Addr, Empty, Api, Storage, BlockInfo, CustomQuery, Querier, Binary};
use cw_multi_test::{Executor, AppResponse, Module, CosmosRouter, BasicAppBuilder};

use crate::asset::TestNativeAsset;
use super::{CustomTestEnv, CustomApp};

pub struct NativeAssetCustomHandler {}
pub type NativeAssetApp = CustomApp<NativeAssetCustomHandler, CustomMsg>;

impl Module for NativeAssetCustomHandler {
    type ExecT = CustomMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC:
            std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {

        println!("custom handler execute");
        println!("{:?}", msg);

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

        let app = BasicAppBuilder::<CustomMsg, Empty>::new_custom()
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
