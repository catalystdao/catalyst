use anyhow::{Result as AnyResult, bail};
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Uint128, Addr};
use cw_multi_test::{App, Executor, AppResponse};
use vault_assets::asset::asset_cw20::Cw20Asset;

use crate::{asset::TestCw20Asset, token::{deploy_test_tokens, get_token_allowance, increase_token_allowance, decrease_token_allowance}};

use super::CustomTestEnv;

pub struct TestCw20AssetEnv(App, Vec<TestCw20Asset>);

impl CustomTestEnv<Cw20Asset, TestCw20Asset> for TestCw20AssetEnv {

    fn initialize(gov: String) -> Self {

        let mut app = App::default();

        let assets = deploy_test_tokens(
            &mut app,
            gov,
            None,
            5
        ).iter().map(|asset| TestCw20Asset(asset.to_string())).collect();

        TestCw20AssetEnv(app, assets)
    }

    fn get_app(&mut self) -> &mut App {
        &mut self.0
    }

    fn get_assets(&self) -> Vec<TestCw20Asset> {
        self.1.to_vec()
    }

    fn execute_contract<U: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &U,
        send_assets: Vec<TestCw20Asset>,
        send_amounts: Vec<Uint128>
    ) -> AnyResult<AppResponse> {

        if send_assets.len() != send_amounts.len() {
            bail!("Assets and amounts len mismatch.".to_string());
        }

        // Get the initial contract allowances
        let initial_allowances: Vec<Uint128> = send_assets.iter()
            .map(|asset| {
                get_token_allowance(
                    self.get_app(),
                    Addr::unchecked(asset.0.clone()),
                    sender.clone(),
                    contract_addr.to_string()
                ).allowance
            })
            .collect();

        // Set allowances
        send_assets.iter()
            .zip(send_amounts)
            .for_each(|(asset, amount)| {
                increase_token_allowance(
                    self.get_app(),
                    amount,
                    Addr::unchecked(asset.0.to_string()),
                    sender.clone(),
                    contract_addr.to_string()
                );

            });

        // Execute contract
        let result = self.get_app().execute_contract(
            sender.clone(),
            contract_addr.clone(),
            msg,
            &[]
        );

        // Reset the contract allowances
        send_assets.iter()
            .zip(initial_allowances)
            .for_each(|(asset, initial_allowance)| {
                let new_allowance = get_token_allowance(
                    self.get_app(),
                    Addr::unchecked(asset.0.clone()),
                    sender.clone(),
                    contract_addr.to_string()
                ).allowance;

                if new_allowance > initial_allowance {
                    decrease_token_allowance(
                        self.get_app(),
                        new_allowance - initial_allowance,
                        Addr::unchecked(asset.0.to_string()),
                        sender.clone(),
                        contract_addr.to_string()
                    );
                }
            });

        result
    }

}
