use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Deps, Uint128, MessageInfo, CosmosMsg, Env, to_binary};
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};
use cw_storage_plus::Item;

use crate::{asset::{AssetTrait, VaultAssetsTrait}, error::AssetError};

const ASSETS: Item<Vec<String>> = Item::new("catalyst-vault-cw20-assets");

pub struct Cw20VaultAssets(pub Vec<Cw20Asset>);

impl<'a> VaultAssetsTrait<'a, Cw20Asset> for Cw20VaultAssets {

    fn new(assets: Vec<Cw20Asset>) -> Self {
        Self(assets)
    }

    fn get_assets(&self) -> &Vec<Cw20Asset> {
        &self.0
    }

    fn load_refs(deps: &Deps) -> Result<Vec<String>, AssetError> {
        ASSETS.load(deps.storage)
            .map_err(|err| err.into())
    }

    fn save_refs(deps: &mut DepsMut, asset_refs: &Vec<String>) -> Result<(), AssetError> {
        ASSETS.save(deps.storage, asset_refs)
            .map_err(|err| err.into())
    }

    fn receive_assets(&self, env: &Env, info: &MessageInfo, amounts: Vec<Uint128>) -> Result<Vec<CosmosMsg>, AssetError> {

        if info.funds.len() != 0 {
            return Err(AssetError::ReceivedAssetCountSurplus {});
        }
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when receiving assets.".to_string()
            })
        }

        // ! IMPORTANT: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // ! transfer messages for zero-valued balance transfers to prevent these cases from 
        // ! resulting in failed transactions.
        let cosmos_messages = self.get_assets()
            .iter()
            .zip(amounts)
            .filter(|(_, balance)| !balance.is_zero())     // Do not create transfer messages for zero-valued transfers
            .map(|(asset, amount)| {
                Ok(CosmosMsg::Wasm(
                    cosmwasm_std::WasmMsg::Execute {
                        contract_addr: asset.0.clone(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.to_string(),
                            recipient: env.contract.address.to_string(),
                            amount
                        })?,
                        funds: vec![]
                    }
                ))
            })
            .collect::<Result<Vec<CosmosMsg>, AssetError>>()?;

        Ok(cosmos_messages)
    }

    fn send_assets(&self, _env: &Env, amounts: Vec<Uint128>, recipient: String) -> Result<Vec<CosmosMsg>, AssetError> {
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when sending assets.".to_string()
            })
        }

        // ! IMPORTANT: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // ! transfer messages for zero-valued balance transfers to prevent these cases from 
        // ! resulting in failed transactions.
        let cosmos_messages = self.get_assets()
            .iter()
            .zip(amounts)
            .filter(|(_, balance)| !balance.is_zero())     // Do not create transfer messages for zero-valued transfers
            .map(|(asset, amount)| {
                Ok(CosmosMsg::Wasm(
                    cosmwasm_std::WasmMsg::Execute {
                        contract_addr: asset.0.to_owned(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: recipient.to_string(),
                            amount
                        })?,
                        funds: vec![]
                    }
                ))
            })
            .collect::<Result<Vec<CosmosMsg>, AssetError>>()?;

        Ok(cosmos_messages)
    }
}



#[cw_serde]
pub struct Cw20Asset(pub String);

impl AssetTrait for Cw20Asset {

    fn get_asset_ref(&self) -> &str {
        &self.0
    }

    fn load(_deps: &Deps, asset_ref: &str) -> Result<Self, AssetError> {

        Ok(Cw20Asset(asset_ref.to_owned()))
    }

    fn save(&self, _deps: &mut DepsMut) -> Result<(), AssetError> {

        Ok(())
    }

    fn query_balance(&self, deps: &Deps, account: impl Into<String>) -> Result<Uint128, AssetError> {
        
        let amount = deps.querier.query_wasm_smart::<BalanceResponse>(
            self.0.to_owned(),
            &Cw20QueryMsg::Balance { address: account.into() }
        )?.balance;

        Ok(amount)
    }

    fn receive_asset(&self, env: &Env, info: &MessageInfo, amount: Uint128) -> Result<Option<CosmosMsg>, AssetError> {

        if info.funds.len() != 0 {
            return Err(AssetError::ReceivedAssetCountSurplus {});
        }

        // ! IMPORTANT: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // ! transfer messages for zero-valued balance transfers to prevent these cases from 
        // ! resulting in failed transactions.
        if amount.is_zero() {
            return Ok(None);
        }

        Ok(Some(CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: self.0.clone(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount
                })?,
                funds: vec![]
            }
        )))
    }

    fn send_asset(&self, _env: &Env, amount: Uint128, recipient: String) -> Result<Option<CosmosMsg>, AssetError> {

        // ! IMPORTANT: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // ! transfer messages for zero-valued balance transfers to prevent these cases from 
        // ! resulting in failed transactions.
        if amount.is_zero() {
            return Ok(None);
        }

        Ok(Some(CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: self.0.to_owned(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient,
                    amount
                })?,
                funds: vec![]
            }
        )))

    }
}

impl ToString for Cw20Asset {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}
