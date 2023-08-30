use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Deps, Uint128, MessageInfo, Coin, CosmosMsg, BankMsg, Env};
use cw_storage_plus::{Item, Map};

use crate::{asset::{AssetTrait, VaultAssetsTrait}, error::AssetError};

const ASSETS_ALIASES: Item<Vec<String>> = Item::new("catalyst-vault-native-assets-aliases");
const ASSETS: Map<&str, String> = Map::new("catalyst-vault-native-assets");


// NOTE: See the `VaultAssetsTrait` and `AssetTrait` definitions for documentation on the
// implemented methods.


/// Vault native asset handler
pub struct NativeVaultAssets(pub Vec<NativeAsset>);

impl<'a> VaultAssetsTrait<'a, NativeAsset> for NativeVaultAssets {


    fn new(assets: Vec<NativeAsset>) -> Self {
        Self(assets)
    }


    fn get_assets(&self) -> &Vec<NativeAsset> {
        &self.0
    }


    fn load_refs(deps: &Deps) -> Result<Vec<String>, AssetError> {
        ASSETS_ALIASES.load(deps.storage)
            .map_err(|err| err.into())
    }


    fn save_refs(&self, deps: &mut DepsMut) -> Result<(), AssetError> {

        let assets_refs = self.get_assets()
            .iter()
            .map(|asset| {
                asset.get_asset_ref().to_owned()
            })
            .collect();

        ASSETS_ALIASES.save(deps.storage, &assets_refs)
            .map_err(|err| err.into())
    }


    fn receive_assets(
        &self,
        _env: &Env,
        info: &MessageInfo,
        amounts: Vec<Uint128>
    ) -> Result<Vec<CosmosMsg>, AssetError> {
        
        // ! **IMPORTANT**: This function assumes that the assets contained within the `NativeVaultAssets`
        // ! struct are unique.
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when receiving assets.".to_string()
            })
        }

        // NOTE: The 'bank' module disallows zero-valued coin transfers. Do not check
        // received funds for these cases.
        let mut non_zero_assets_count = 0;

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
                                reason: format!(
                                    "Received {}, expected {}",
                                    coin,
                                    Coin::new(amount.u128(), asset.denom.to_owned())
                                )
                            })
                        }
                        else {
                            Ok(())
                        }
                    },
                    None => Err(AssetError::ReceivedAssetInvalid {
                        reason: format!(
                            "{} not received",
                            Coin::new(amount.u128(), asset.denom.to_owned())
                        )
                    })
                }
            })?;

            let received_funds_count = info.funds.len();

            // NOTE: There is no need to check whether 'received_funds_count < non_zero_assets_count',
            // as in that case the check above would have failed for at least one of the expected assets
            // (assuming all assets contained by the vault are unique).
        
            if received_funds_count > non_zero_assets_count {
                return Err(AssetError::ReceivedAssetCountSurplus {});
            }

        Ok(vec![])
    }


    fn send_assets(
        &self,
        _env: &Env,
        amounts: Vec<Uint128>,
        recipient: String
    ) -> Result<Vec<CosmosMsg>, AssetError> {
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when sending assets.".to_string()
            })
        }

        // NOTE: The 'bank' module disallows zero-valued coin transfers. Do not generate
        // transfer orders for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
        let transfer_amounts: Vec<Coin> = self.get_assets()
            .iter()
            .zip(amounts)
            .filter(|(_, amount)| !amount.is_zero())     // Do not create transfer orders for zero-valued transfers
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.to_owned())
            })
            .collect();

        if transfer_amounts.len() == 0 {
            return Ok(vec![]);
        }

        let cosmos_message = CosmosMsg::Bank(BankMsg::Send {
            to_address: recipient.clone(),
            amount: transfer_amounts
        });

        Ok(vec![cosmos_message])
    }

}



/// Native asset handler
/// 
/// NOTE: For native assets, the asset *reference* is the asset *alias*.
/// 
#[cw_serde]
pub struct NativeAsset {
    pub denom: String,
    pub alias: String
}

impl AssetTrait for NativeAsset {

    fn from_asset_ref(deps: &Deps, asset_ref: &str) -> Result<Self, AssetError> {
        
        let denom = match ASSETS.load(deps.storage, asset_ref) {
            Ok(denom) => denom,
            Err(_) => return Err(AssetError::AssetNotFound {}),
        };

        Ok(NativeAsset {
            denom,
            alias: asset_ref.to_string()
        })
    }


    fn get_asset_ref(&self) -> &str {
        &self.alias
    }


    fn save(&self, deps: &mut DepsMut) -> Result<(), AssetError> {

        let asset_ref = self.get_asset_ref();

        ASSETS.save(deps.storage, &asset_ref, &self.denom)?;

        Ok(())
    }


    fn query_prior_balance(
        &self,
        deps: &Deps,
        env: &Env,
        info: Option<&MessageInfo>
    ) -> Result<Uint128, AssetError> {

        // ! **IMPORTANT**: For native assets, the *prior balance* is **NOT** the real current
        // ! balance. Any received assets are subtracted from the real current balance, as
        // ! *received* native assets are processed **before** the message execution.
        
        let queried_balance = deps.querier.query_balance(
            env.contract.address.to_string(),
            self.denom.to_string()
        )?.amount;

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
                    queried_balance
                        .checked_sub(funds) // 'checked_sub' used for extra precaution ('wrapping_sub' should be sufficient).
                        .map_err(|err| AssetError::Std(err.into()))?
                )
                
            },
            None => {
                Ok(queried_balance)
            }
        }

    }


    fn receive_asset(
        &self,
        _env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<CosmosMsg>, AssetError> {

        // NOTE: The 'bank' module disallows zero-valued coin transfers. Do not check
        // received funds for these cases.
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

    fn send_asset(
        &self,
        _env: &Env,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<CosmosMsg>, AssetError> {

        // NOTE: The 'bank' module disallows zero-valued coin transfers. Do not generate
        // transfer orders for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
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
        format!("{} (alias: {})", self.denom, self.alias)
    }
}
