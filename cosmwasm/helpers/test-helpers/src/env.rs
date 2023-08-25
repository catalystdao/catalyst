use anyhow::Result as AnyResult;
use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::{Uint128, Addr};
use cw_multi_test::{App, AppResponse};
use vault_assets::asset::AssetTrait;

use crate::asset::CustomTestAsset;

pub mod env_cw20_asset;
pub mod env_native_asset;

pub trait CustomTestEnv<A: AssetTrait, T: CustomTestAsset<A>> {

    fn initialize(gov: String) -> Self;

    fn get_app(&mut self) -> &mut App;

    fn get_assets(&self) -> Vec<T>;

    fn execute_contract<U: Serialize + std::fmt::Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &U,
        send_assets: Vec<T>,
        send_amounts: Vec<Uint128>
    ) -> AnyResult<AppResponse>;

}