use anyhow::Result as AnyResult;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Uint128, Addr};
use cw_multi_test::{App, AppResponse};
use vault_assets::asset::AssetTrait;

use crate::asset::CustomTestAsset;

pub mod env_cw20_asset;
pub mod env_native_asset;

/// Helper around `cw_multi_test::App` to allow configurable contract testing.
pub trait CustomTestEnv<A: AssetTrait, T: CustomTestAsset<A>> {

    /// Setup the test 'environment'. This includes creating mock assets according to the
    /// desired asset type (native assets/cw20).
    /// 
    /// # Arguments:
    /// * `gov` - The account to hold the created assets.
    /// 
    fn initialize(gov: String) -> Self;


    /// Get a reference to `cw_multi_test::App`.
    fn get_app(&mut self) -> &mut App;


    /// Get the mock assets.
    fn get_assets(&self) -> Vec<T>;


    /// Execute a contract with the specified funds.
    /// - Native assets: the funds are specified on the contract execution.
    /// - CW20 assets: token allowances are set for the invoked contract.
    fn execute_contract<U: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &U,
        send_assets: Vec<T>,
        send_amounts: Vec<Uint128>
    ) -> AnyResult<AppResponse>;

}