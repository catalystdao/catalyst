use cosmwasm_std::{Uint128, CosmosMsg, Deps, DepsMut, MessageInfo, Env};
use serde::Serialize;
use std::fmt::Debug;

use crate::error::AssetError;

pub mod asset_native;
pub mod asset_cw20;


/// Trait defining the interface of the vault assets handler struct.
pub trait VaultAssetsTrait<'a, T: AssetTrait + 'a> {

    /// Generate a new vault assets handler with the specified assets.
    /// 
    /// ! **IMPORTANT**: This function does not check the validity of the provided assets.
    /// 
    /// # Arguments:
    /// * `assets` - The assets contained by the vault.
    /// 
    fn new(assets: Vec<T>) -> Self;


    /// Get the vault assets.
    fn get_assets(&self) -> &Vec<T>;


    /// Get the references of the vault assets.
    fn get_assets_refs(&'a self) -> Vec<&'a str> {
        
        self.get_assets()
            .iter()
            .map(|asset: &'a T| {
                asset.get_asset_ref()
            })
            .collect::<Vec<&'a str>>()

    }


    /// Load only the asset references from storage.
    /// 
    /// **NOTE**: This is a 'cheaper' alternative to `load_assets`, as only the asset 
    /// references are loaded from storage.
    /// 
    fn load_refs(deps: &Deps) -> Result<Vec<String>, AssetError>;


    /// Save only the asset references to storage.
    /// 
    /// ! **IMPORTANT**: This function should not be used on its own, `save` should be used instead.
    /// 
    fn save_refs(&self, deps: &mut DepsMut) -> Result<(), AssetError>;


    /// Load the handler from storage.
    /// 
    /// **NOTE**: Use `load_refs` instead if only the asset references are required. This function
    /// loads the asset references via `load_refs` **and then** loads the entire handler using the
    /// references.
    ///  
    fn load(deps: &Deps) -> Result<Self, AssetError> where Self: Sized {

        let assets_refs = Self::load_refs(deps)?;

        Ok(
            Self::new(
                assets_refs.iter()
                    .map(|asset_ref| T::from_asset_ref(deps, asset_ref))
                    .collect::<Result<Vec<T>, AssetError>>()?
            )
        )
    }


    /// Save the handler assets to storage.
    fn save(&self, deps: &mut DepsMut) -> Result<(), AssetError> {

        self.save_refs(deps)?;

        self.get_assets()
            .iter()
            .try_for_each(|asset| {
                asset.save(deps)
            })?;

        Ok(())
    }


    /// Verify that no assets are received within the message execution.
    fn receive_no_assets(info: &MessageInfo) -> Result<(), AssetError> {

        if info.funds.len() > 0 {
            Err(AssetError::AssetSurplusReceived {})
        }
        else {
            Ok(())
        }

    }


    /// Receive the specified amounts of the vault assets within the message execution.
    /// 
    /// NOTE: May return `CosmosMsg`s to order the transfer of the assets.
    /// 
    /// # Arguments:
    /// * `amounts` - The amounts of the assets to receive.
    /// 
    fn receive_assets(
        &self,
        env: &Env,
        info: &MessageInfo,
        amounts: Vec<Uint128>
    ) -> Result<Vec<CosmosMsg>, AssetError>;


    /// Send the specified amounts of the vault assets within the message execution.
    /// 
    /// NOTE: Always returns `CosmosMsg`s to order the transfer of the assets except for
    /// zero-valued amounts.
    /// 
    /// # Arguments:
    /// * `amounts` - The amounts of the assets to send.
    /// * `recipient` - The recipient of the assets
    /// 
    fn send_assets(&self,
        env: &Env,
        amounts: Vec<Uint128>,
        recipient: String
    ) -> Result<Vec<CosmosMsg>, AssetError>;

}


/// Trait defining the interface of the individual vault assets.
pub trait AssetTrait: Serialize + PartialEq + Debug + Clone + ToString {

    /// Get the asset corresponding to a specific asset_ref.
    /// 
    /// ! **IMPORTANT**: This method by itself does no guarantee whether the asset exists/is part
    /// of the vault.
    /// 
    /// # Arguments:
    /// * `asset_ref` - The asset reference.
    /// 
    fn from_asset_ref(deps: &Deps, asset_ref: &str) -> Result<Self, AssetError>;


    /// Get the asset reference.
    fn get_asset_ref(&self) -> &str;


    /// Save the asset details to storage (if any).
    /// 
    /// ! **IMPORTANT**: This function should not be used on its own. Asset saving should be
    /// executed via the global vault asset handling struct.
    /// 
    fn save(&self, deps: &mut DepsMut) -> Result<(), AssetError>;


    /// Query the vault's **effective** asset balance **before the start** of the current message
    /// execution.
    /// 
    /// ! **IMPORTANT**: This method does not necessarily return the **real** current balance, but
    /// rather returns the vault's balance **without taking into account any incoming funds**.
    /// 
    fn query_prior_balance(
        &self,
        deps: &Deps,
        env: &Env,
        info: Option<&MessageInfo>
    ) -> Result<Uint128, AssetError>;


    /// Receive the specified amount of the asset within the message execution.
    /// 
    /// NOTE: May return a `CosmosMsg` to order the transfer of the assets.
    /// 
    /// # Arguments:
    /// * `amount` - The asset amount to receive
    /// 
    fn receive_asset(&self,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<CosmosMsg>, AssetError>;


    /// Send the specified amount of the asset to a recipient within the message execution.
    /// 
    /// NOTE: Always returns a `CosmosMsg` to order the transfer of the assets except for
    /// zero-valued amounts.
    /// 
    /// # Arguments:
    /// * `amount` - The asset amount to receive
    /// 
    fn send_asset(&self,
        env: &Env,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<CosmosMsg>, AssetError>;

}

