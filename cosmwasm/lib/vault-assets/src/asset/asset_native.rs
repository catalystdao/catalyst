use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Deps, Uint128, MessageInfo, Coin, CosmosMsg, BankMsg, Env};
use cw_storage_plus::{Item, Map};

use crate::{asset::{AssetTrait, VaultAssetsTrait}, error::AssetError};

const ASSETS: Item<Vec<String>> = Item::new("catalyst-vault-native-assets");
const ASSETS_ALIASES: Map<&str, String> = Map::new("catalyst-vault-native-assets-aliases");

pub struct NativeVaultAssets(pub Vec<NativeAsset>);

impl<'a> VaultAssetsTrait<'a, NativeAsset> for NativeVaultAssets {

    //TODO rename new_unchecked?
    fn new(assets: Vec<NativeAsset>) -> Self {
        Self(assets)
    }

    // TODO rename get_vec?
    fn get_assets(&self) -> &Vec<NativeAsset> {
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

    fn receive_assets(&self, _env: &Env, info: &MessageInfo, amounts: Vec<Uint128>) -> Result<Vec<CosmosMsg>, AssetError> {
        
        //!NOTE: This function assumes that the assets contained within the `NativeVaultAssets` struct are unique.
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when receiving assets.".to_string()
            })
        }

        let mut non_zero_assets_count = 0;
        
        //TODO better way to do this?
        self.get_assets()
            .iter()
            .zip(amounts)
            .filter(|(_, amount)| !amount.is_zero())    // Bank transfers do not allow zero-valued amounts
            .try_for_each(|(asset, amount)| -> Result<(), AssetError> {

                non_zero_assets_count += 1;

                let received_coin = info.funds.iter().find(|coin| {
                    coin.denom == asset.denom
                });

                match received_coin {
                    Some(coin) => {
                        if coin.amount != amount {
                            Err(AssetError::ReceivedAssetInvalid {
                                reason: format!("Received {}, expected {}", coin, Coin::new(amount.u128(), asset.denom.to_owned()))
                            })
                        }
                        else {
                            Ok(())
                        }
                    },
                    None => Err(AssetError::ReceivedAssetInvalid {
                        reason: format!("{} not received", Coin::new(amount.u128(), asset.denom.to_owned()))
                    })
                }
            })?;

            let received_funds_count = info.funds.len();

            // NOTE: There is no need to check whether 'received_funds_count < non_zero_assets_count',
            // as in that case the check above would have failed for at least one of the expected assets
            // (assuming all assets contained are unique).
        
            if received_funds_count > non_zero_assets_count {
                return Err(AssetError::ReceivedAssetCountSurplus {});
            }

        Ok(vec![])
    }

    fn send_assets(&self, _env: &Env, amounts: Vec<Uint128>, recipient: String) -> Result<Vec<CosmosMsg>, AssetError> {
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when sending assets.".to_string()
            })
        }

        let cosmos_messages = self.get_assets()
            .iter()
            .zip(amounts)
            .filter(|(_, amount)| !amount.is_zero())    // Bank transfers do not allow zero-valued amounts
            .map(|(asset, amount)| {
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: recipient.clone(),
                    amount: vec![Coin::new(amount.u128(), asset.denom.to_owned())]
                })
            })
            .collect();

        Ok(cosmos_messages)
    }
}



#[cw_serde]
pub struct NativeAsset {
    pub denom: String,
    pub alias: String
}

impl AssetTrait for NativeAsset {

    fn get_asset_ref(&self) -> &str {
        &self.alias
    }

    fn load(deps: &Deps, asset_ref: &str) -> Result<Self, AssetError> {
        
        let denom = match ASSETS_ALIASES.load(deps.storage, asset_ref) {
            Ok(denom) => denom,
            Err(_) => return Err(AssetError::AssetNotFound {}),
        };

        Ok(NativeAsset {
            denom,
            alias: asset_ref.to_string()
        })
    }

    fn save(&self, deps: &mut DepsMut) -> Result<(), AssetError> {

        let asset_ref = self.get_asset_ref();

        ASSETS_ALIASES.save(deps.storage, &asset_ref, &self.denom)?;

        Ok(())
    }

    fn query_prior_balance(&self, deps: &Deps, info: Option<&MessageInfo>, account: impl Into<String>) -> Result<Uint128, AssetError> {
        
        let amount = deps.querier.query_balance(account, self.denom.to_string())?.amount;

        let incoming_funds = match info {
            Some(info) => {
                info.funds.iter()
                    .find(|coin| coin.denom == self.denom)
                    .and_then(|coin| Some(coin.amount))
            },
            None => None,
        };

        match incoming_funds {
            Some(funds) => {
                Ok(
                    amount
                        .checked_sub(funds)
                        .map_err(|err| AssetError::Std(err.into()))?
                )
                
            },
            None => {
                Ok(amount)
            }
        }

    }

    fn receive_asset(&self, _env: &Env, info: &MessageInfo, amount: Uint128) -> Result<Option<CosmosMsg>, AssetError> {

        if amount.is_zero() {
            if info.funds.len() != 0 {
                return Err(AssetError::ReceivedAssetCountSurplus {});
            }
            
            return Ok(None);
        }

        match info.funds.len() {
            0 => Err(AssetError::ReceivedAssetCountShortage {}),
            1 => {
                let received_coin = info.funds[0].to_owned();
                let expected_coin = Coin::new(amount.u128(), self.denom.to_owned());
                if received_coin != expected_coin {
                    Err(AssetError::ReceivedAssetInvalid {
                        reason: format!("Received {}, expected {}", received_coin, expected_coin)
                    })
                }
                else {
                    Ok(None)
                }
            },
            _ => Err(AssetError::ReceivedAssetCountSurplus {})
        }
    }

    fn send_asset(&self, _env: &Env, amount: Uint128, recipient: String) -> Result<Option<CosmosMsg>, AssetError> {

        if amount.is_zero() {
            return Ok(None);
        }

        Ok(Some(CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient,
            amount: vec![Coin::new(amount.u128(), self.denom.clone())]
        })))
    }
}

impl ToString for NativeAsset {
    fn to_string(&self) -> String {
        format!("{} (alias: {})", self.denom, self.alias)   //TODO overhaul
    }
}