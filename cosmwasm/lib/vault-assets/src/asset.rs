use cosmwasm_std::{Uint128, CosmosMsg, Deps, DepsMut, MessageInfo, Env};
use serde::Serialize;
use std::fmt::Debug;

use crate::error::AssetError;

#[cfg(all(not(feature="asset_native"), not(feature="asset_cw20")))]
compile_error!("An asset-type feature must be enabled (\"asset_native\" or \"asset_cw20\")");

#[cfg(all(feature="asset_native", feature="asset_cw20"))]
compile_error!("Multiple asset-type features cannot be enabled at the same time (\"asset_native\" and \"asset_cw20\")");

#[cfg(feature="asset_native")]
mod asset_native;
#[cfg(feature="asset_native")]
pub use asset_native::NativeAsset as Asset;
#[cfg(feature="asset_native")]
pub use asset_native::NativeVaultAssets as VaultAssets;

#[cfg(feature="asset_cw20")]
mod asset_cw20;
#[cfg(feature="asset_cw20")]
pub use asset_cw20::Cw20Asset as Asset;
#[cfg(feature="asset_cw20")]
pub use asset_cw20::Cw20VaultAssets as VaultAssets;



pub trait VaultAssetsTrait<'a, T: AssetTrait + 'a> {

    fn new(assets: Vec<T>) -> Self;

    fn get_assets(&self) -> &Vec<T>;

    fn get_assets_refs(&'a self) -> Vec<&'a str> {
        
        self.get_assets()
            .iter()
            .map(|asset: &'a T| {
                asset.get_asset_ref()
            })
            .collect::<Vec<&'a str>>()

    }


    fn load_refs(deps: &Deps) -> Result<Vec<String>, AssetError>;

    fn save_refs(deps: &mut DepsMut, asset_refs: &Vec<String>) -> Result<(), AssetError>;


    fn load_assets(deps: &Deps) -> Result<Self, AssetError> where Self: Sized {

        let assets_refs = Self::load_refs(deps)?;
        
        Self::load_assets_from_refs(deps, assets_refs)
    }

    fn load_assets_from_refs(deps: &Deps, assets_refs: Vec<String>) -> Result<Self, AssetError> where Self: Sized {

        Ok(
            Self::new(
                assets_refs.iter()
                    .map(|asset_ref| T::load(deps, asset_ref))
                    .collect::<Result<Vec<T>, AssetError>>()?
                )
        )

    }

    fn save_assets(&self, deps: &mut DepsMut) -> Result<(), AssetError> {

        let assets_refs = self.get_assets()
            .iter()
            .map(|asset| {
                asset.save(deps)?;
                Ok(asset.get_asset_ref().to_owned())
            })
            .collect::<Result<Vec<String>, AssetError>>()?;

        Self::save_refs(deps, &assets_refs)?;

        Ok(())
    }


    fn receive_no_assets(info: &MessageInfo) -> Result<(), AssetError> {
        if info.funds.len() > 0 {
            Err(AssetError::ReceivedAssetCountSurplus {})
        }
        else {
            Ok(())
        }
    }

    fn receive_assets(&self, env: &Env, info: &MessageInfo, amounts: Vec<Uint128>) -> Result<Vec<CosmosMsg>, AssetError>;

    fn send_assets(&self, env: &Env, amounts: Vec<Uint128>, recipient: String) -> Result<Vec<CosmosMsg>, AssetError>;


}

pub trait AssetTrait: Serialize + PartialEq + Debug + Clone + ToString {

    fn get_asset_ref(&self) -> &str;

    fn load(deps: &Deps, asset_ref: &str) -> Result<Self, AssetError>;

    fn save(&self, deps: &mut DepsMut) -> Result<(), AssetError>;

    fn query_balance(&self, deps: &Deps, account: impl Into<String>) -> Result<Uint128, AssetError>;

    //TODO replace &MessageInfo with 'sender'? (match `send_asset` structure)
    fn receive_asset(&self, env: &Env, info: &MessageInfo, amount: Uint128) -> Result<Option<CosmosMsg>, AssetError>;

    //TODO use 'into<String>' instead of 'String' for recipient
    fn send_asset(&self, env: &Env, amount: Uint128, recipient: String) -> Result<Option<CosmosMsg>, AssetError>;

}

