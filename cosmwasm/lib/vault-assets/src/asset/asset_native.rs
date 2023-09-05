use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Deps, Uint128, MessageInfo, Coin, BankMsg, Env};
use cw_storage_plus::{Item, Map};

use crate::{asset::{AssetTrait, VaultAssetsTrait}, error::AssetError};

const ASSETS_ALIASES: Item<Vec<String>> = Item::new("catalyst-vault-native-assets-aliases");
const ASSETS: Map<&str, String> = Map::new("catalyst-vault-native-assets");


// NOTE: See the `VaultAssetsTrait` and `AssetTrait` definitions for documentation on the
// implemented methods.


#[cw_serde]
pub enum NativeAssetMsg {
    Bank(BankMsg)
}


/// Vault native asset handler
pub struct NativeVaultAssets(pub Vec<NativeAsset>);

impl<'a> VaultAssetsTrait<'a, NativeAsset, NativeAssetMsg> for NativeVaultAssets {


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
    ) -> Result<Vec<NativeAssetMsg>, AssetError> {
        
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
                            Err(AssetError::UnexpectedAssetAmountReceived {
                                received_amount: coin.amount,
                                expected_amount: amount,
                                asset: asset.to_string()
                            })
                        }
                        else {
                            Ok(())
                        }
                    },
                    None => Err(AssetError::AssetNotReceived {
                        asset: asset.to_string()
                    })
                }
            })?;

            let received_funds_count = info.funds.len();

            // NOTE: There is no need to check whether 'received_funds_count < non_zero_assets_count',
            // as in that case the check above would have failed for at least one of the expected assets
            // (assuming all assets contained by the vault are unique).
        
            if received_funds_count > non_zero_assets_count {
                return Err(AssetError::AssetSurplusReceived {});
            }

        Ok(vec![])
    }


    fn send_assets(
        &self,
        _env: &Env,
        amounts: Vec<Uint128>,
        recipient: String
    ) -> Result<Vec<NativeAssetMsg>, AssetError> {
        
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

        let message = NativeAssetMsg::Bank(BankMsg::Send {
            to_address: recipient.clone(),
            amount: transfer_amounts
        });

        Ok(vec![message])
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

impl AssetTrait<NativeAssetMsg> for NativeAsset {

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
    ) -> Result<Option<NativeAssetMsg>, AssetError> {

        // NOTE: The 'bank' module disallows zero-valued coin transfers. Do not check
        // received funds for these cases.
        if amount.is_zero() {
            if info.funds.len() != 0 {
                return Err(AssetError::AssetSurplusReceived {});
            }
            
            return Ok(None);
        }

        match info.funds.len() {
            0 => Err(AssetError::AssetNotReceived { asset: self.to_string() }),
            1 => {

                if info.funds[0].denom != self.denom {
                    return Err(AssetError::AssetNotReceived {
                        asset: self.to_string()
                    })
                }

                if info.funds[0].amount != amount {
                    return Err(AssetError::UnexpectedAssetAmountReceived {
                        received_amount: info.funds[0].amount,
                        expected_amount: amount,
                        asset: self.to_string()
                    });
                }

                Ok(None)
            },
            _ => Err(AssetError::AssetSurplusReceived {})
        }
    }

    fn send_asset(
        &self,
        _env: &Env,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<NativeAssetMsg>, AssetError> {

        // NOTE: The 'bank' module disallows zero-valued coin transfers. Do not generate
        // transfer orders for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
        if amount.is_zero() {
            return Ok(None);
        }

        Ok(Some(NativeAssetMsg::Bank(BankMsg::Send {
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



#[cfg(test)]
mod asset_native_tests {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Uint128, Coin};

    use crate::{asset::{VaultAssetsTrait, AssetTrait, asset_native::NativeAssetMsg}, error::AssetError};

    use super::{NativeVaultAssets, NativeAsset};

    const SENDER_ADDR   : &str = "sender_addr";
    const RECEIVER_ADDR : &str = "receiver_addr";

    
    fn get_mock_asset() -> NativeAsset {
        NativeAsset {
            denom: "asset_a".to_string(),
            alias: "a".to_string()
        }
    }

    
    fn get_mock_assets() -> Vec<NativeAsset> {
        vec![
            NativeAsset {
                denom: "asset_a".to_string(),
                alias: "a".to_string()
            },
            NativeAsset {
                denom: "asset_b".to_string(),
                alias: "b".to_string()
            },
            NativeAsset {
                denom: "asset_c".to_string(),
                alias: "c".to_string()
            }
        ]
    }



    // Handler tests
    // ********************************************************************************************

    #[test]
    fn test_new_vault_assets_handler() {

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());

        assert_eq!(
            handler.get_assets().to_owned(),
            assets
        )
    }


    #[test]
    fn test_save_and_load_vault_assets_handler() {

        let mut deps = mock_dependencies();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());



        // Tested action 1: save handler
        // NOTE: `save_refs` is tested indirectly via the `save` method.
        handler.save(&mut deps.as_mut()).unwrap();



        // Tested action 2: load references only
        let loaded_refs = NativeVaultAssets::load_refs(&deps.as_ref()).unwrap();
        assert_eq!(
            loaded_refs,
            assets.iter().map(|asset| asset.get_asset_ref().to_owned()).collect::<Vec<String>>()
        );



        // Tested action 3: load the entire handler
        let loaded_handler = NativeVaultAssets::load(&deps.as_ref()).unwrap();

        // Make sure the loaded assets match the saved ones
        assert_eq!(
            loaded_handler.get_assets().to_owned(),
            assets.clone()
        );

    }


    #[test]
    fn test_handler_receive_assets() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());

        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128),
            Uint128::from(789u128)
        ];

        let received_coins: Vec<Coin> = assets.iter()
            .zip(&desired_received_amounts)
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.clone())
            })
            .collect();



        // Tested action: receive assets
        let msgs = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &received_coins
            ),
            desired_received_amounts.clone()
        ).unwrap();     // Call is successful



        // Verify no messages are generated
        assert!(msgs.len() == 0)

    }


    #[test]
    fn test_handler_receive_asset_invalid_amounts_count() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());

        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128)      // One amount less than assets hold by the vault
        ];



        // Tested action: receive assets with invalid 'amounts' count
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]    // No need to provide coins, should error before this is checked
            ),
            desired_received_amounts.clone()
        );



        // Make sure the call errors
        matches!(
            result.err().unwrap(),
            AssetError::InvalidParameters { reason }
                if reason == "Invalid 'amounts' count when receiving assets.".to_string()
        );

    }


    #[test]
    fn test_handler_receive_assets_invalid_funds() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());

        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128),
            Uint128::from(789u128)
        ];

        let valid_received_coins: Vec<Coin> = assets.iter()
            .zip(&desired_received_amounts)
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.clone())
            })
            .collect();



        // Tested action 1: no funds
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]             // No funds
            ),
            desired_received_amounts.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetNotReceived { asset }
                if asset == assets[0].to_string()   // Error is for the first asset not found
        );



        // Tested action 2: too few assets
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &valid_received_coins[..2]     // One asset less than expected
            ),
            desired_received_amounts.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetNotReceived { asset }
                if asset == assets[2].to_string()   // Error is for the first asset not found
        );



        // Tested action 3: too many assets
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                valid_received_coins.iter()    // One asset more than expected
                    .cloned()
                    .chain(vec![Coin::new(99u128, "other_coin")].into_iter())
                    .collect::<Vec<Coin>>().as_slice()
            ),
            desired_received_amounts.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetSurplusReceived {}
        );



        // Tested action 4: invalid asset
        let mut received_coins_invalid_asset = valid_received_coins.clone();
        received_coins_invalid_asset[2] = Coin::new(99u128, "other_coin");  // Replace last coin
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &received_coins_invalid_asset    // Last expected asset different
            ),
            desired_received_amounts.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetNotReceived { asset }
                if asset == assets[2].to_string()   // Error is for the first asset not found
        );



        // Tested action 4: asset amount too small
        let mut received_coins_small_amount = valid_received_coins.clone();
        received_coins_small_amount[1].amount = received_coins_small_amount[1].amount - Uint128::one();
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &received_coins_small_amount    // Amount too small for the second asset
            ),
            desired_received_amounts.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::UnexpectedAssetAmountReceived {asset, received_amount, expected_amount }
                if asset == assets[1].to_string()
                    && received_amount == received_coins_small_amount[1].amount
                    && expected_amount == valid_received_coins[1].amount
        );



        // Tested action 5: asset amount too large
        let mut received_coins_large_amount = valid_received_coins.clone();
        received_coins_large_amount[1].amount = received_coins_large_amount[1].amount + Uint128::one();
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &received_coins_large_amount    // Amount too large for the second asset
            ),
            desired_received_amounts.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::UnexpectedAssetAmountReceived {asset, received_amount, expected_amount }
                if asset == assets[1].to_string()
                    && received_amount == received_coins_large_amount[1].amount
                    && expected_amount == valid_received_coins[1].amount
        );



        // Make sure 'receive' works for valid funds
        handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &valid_received_coins    // Valid funds
            ),
            desired_received_amounts.clone()
        ).unwrap();

    }


    #[test]
    fn test_handler_receive_assets_zero_amount() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());



        // Tested action 1: one asset with zero amount
        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::zero(),        // Zero amount
            Uint128::from(789u128)
        ];

        let received_coins: Vec<Coin> = assets.iter()
            .zip(&desired_received_amounts)
            .filter(|(_, amount)| !amount.is_zero())    // Bank never sends zero-valued amounts
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.clone())
            })
            .collect();

        handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &received_coins
            ),
            desired_received_amounts.clone()
        ).unwrap();     // Make sure result is successful



        // Tested action 2: all assets with zero amount
        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::zero(),
            Uint128::zero(),
            Uint128::zero()
        ];

        let received_coins: Vec<Coin> = vec![];    // Bank never sends zero-valued amounts

        handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &received_coins
            ),
            desired_received_amounts.clone()
        ).unwrap();     // Make sure result is successful

    }


    #[test]
    fn test_handler_send_assets() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());

        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128),
            Uint128::from(789u128)
        ];



        // Tested action: send assets
        let msgs = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        ).unwrap();



        // Verify that the generated messages are valid
        assert!(msgs.len() == 1);
        let msg = msgs[0].clone();

        let expected_sent_coins: Vec<Coin> = assets.iter()
            .zip(&desired_send_amounts)
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.to_owned())
            })
            .collect();

        matches!(
            msg,
            NativeAssetMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount })
            if to_address == SENDER_ADDR.to_string()
                && amount == expected_sent_coins
        );

    }


    #[test]
    fn test_handler_send_assets_invalid_amounts_count() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());

        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128)      // One amount less than assets hold by the vault
        ];



        // Tested action: send assets with invalid 'amounts' count
        let result = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        );



        // Verify that the generated messages are valid
        matches!(
            result.err().unwrap(),
            AssetError::InvalidParameters { reason }
                if reason == "Invalid 'amounts' count when sending assets.".to_string()
        );

    }


    #[test]
    fn test_handler_send_assets_zero_amount() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = NativeVaultAssets::new(assets.clone());



        // Tested action 1: one asset with zero amount
        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::zero(),        // Zero amount
            Uint128::from(789u128)
        ];

        let msgs = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        ).unwrap();     // Make sure result is successful

        // Verify that the generated messages are valid
        assert!(msgs.len() == 1);
        let msg = msgs[0].clone();

        let expected_sent_coins: Vec<Coin> = assets.iter()
            .zip(&desired_send_amounts)
            .filter(|(_, amount)| !amount.is_zero())    // No coins specified for zero-valued transfers
            .map(|(asset, amount)| {
                Coin::new(amount.u128(), asset.denom.to_owned())
            })
            .collect();

        matches!(
            msg,
            NativeAssetMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount })
            if to_address == SENDER_ADDR.to_string()
                && amount == expected_sent_coins
        );



        // Tested action 2: all assets with zero amount
        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::zero(),
            Uint128::zero(),
            Uint128::zero()
        ];

        let msgs = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        ).unwrap();     // Make sure result is successful

        // Verify that no messages are generated
        assert!(msgs.len() == 0);

    }




    // Asset tests
    // ********************************************************************************************

    #[test]
    fn test_save_and_load_asset() {

        let mut deps = mock_dependencies();

        let asset = NativeAsset {
            denom: "denom".to_string(),
            alias: "alias".to_string(),
        };



        // Tested action 1: Save asset
        asset.save(&mut deps.as_mut()).unwrap();



        // Tested action 2: load asset using its ref
        let loaded_asset = NativeAsset::from_asset_ref(
            &deps.as_ref(),
            asset.get_asset_ref()
        ).unwrap();



        // Verify the loaded asset matches the original asset definition
        assert_eq!(
            loaded_asset,
            asset
        )
    }


    #[test]
    fn test_asset_ref() {

        let asset = NativeAsset {
            denom: "denom".to_string(),
            alias: "alias".to_string(),
        };



        // Tested action: get the asset ref
        let asset_ref = asset.get_asset_ref();


        
        // Verify the asset ref is the asset alias
        assert_eq!(
            asset_ref.to_string(),
            asset.alias
        )
    }

    
    // TODO query


    #[test]
    fn test_receive_asset() {

        let env = mock_env();

        let asset = get_mock_asset();
        let desired_received_amount = Uint128::from(123_u128);
        let received_coin = Coin::new(
            desired_received_amount.u128(),
            asset.denom.clone()
        );



        // Tested action: receive asset
        let msg = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[received_coin]
            ),
            desired_received_amount.clone()
        ).unwrap();     // Call is successful



        // Verify no messages are generated
        assert!(msg.is_none())

    }


    #[test]
    fn test_receive_asset_invalid_funds() {

        let env = mock_env();

        let asset = get_mock_asset();
        let desired_received_amount = Uint128::from(123_u128);
        let valid_received_coin = Coin::new(
            desired_received_amount.u128(),
            asset.denom.clone()
        );



        // Tested action 1: no funds
        let result = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]             // No funds
            ),
            desired_received_amount.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetNotReceived { asset: error_asset }
                if error_asset == asset.to_string()
        );



        // Tested action 2: too many assets
        let result = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[valid_received_coin.clone(), Coin::new(99u128, "other_coin")]    // One asset more than expected
            ),
            desired_received_amount.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetSurplusReceived {}
        );



        // Tested action 3: invalid asset
        let result = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[Coin::new(99u128, "other_coin")]    // Different asset
            ),
            desired_received_amount.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::AssetNotReceived { asset: error_asset }
                if error_asset == asset.to_string()
        );



        // Tested action 4: asset amount too small
        let mut received_coin_small_amount = valid_received_coin.clone();
        received_coin_small_amount.amount = received_coin_small_amount.amount - Uint128::one();
        let result = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[received_coin_small_amount.clone()]    // Amount too small
            ),
            desired_received_amount.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::UnexpectedAssetAmountReceived {asset: error_asset, received_amount, expected_amount }
                if error_asset == asset.to_string()
                    && received_amount == received_coin_small_amount.amount
                    && expected_amount == valid_received_coin.amount
        );



        // Tested action 5: asset amount too large
        let mut received_coin_large_amount = valid_received_coin.clone();
        received_coin_large_amount.amount = received_coin_large_amount.amount + Uint128::one();
        let result = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[received_coin_large_amount.clone()]    // Amount too large
            ),
            desired_received_amount.clone()
        );

        // Make sure action fails
        matches!(
            result.err().unwrap(),
            AssetError::UnexpectedAssetAmountReceived {asset: error_asset, received_amount, expected_amount }
                if error_asset == asset.to_string()
                    && received_amount == received_coin_large_amount.amount
                    && expected_amount == valid_received_coin.amount
        );


    
        // Make sure 'receive' works for valid funds
        asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[valid_received_coin]
            ),
            desired_received_amount.clone()
        ).unwrap();     // Call is successful

    }


    #[test]
    fn test_receive_asset_zero_amount() {

        let env = mock_env();

        let asset = get_mock_asset();
        let desired_received_amount = Uint128::zero();



        // Tested action: receive zero amount
        let result = asset.receive_asset(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]
            ),
            desired_received_amount
        );



        // Verify action passes
        assert!(result.is_ok());
        assert!(result.ok().unwrap().is_none());

    }


    #[test]
    fn test_send_asset() {

        let env = mock_env();

        let asset = get_mock_asset();
        let desired_send_amount = Uint128::from(123_u128);



        // Tested action: send asset
        let msg = asset.send_asset(
            &env,
            desired_send_amount,
            RECEIVER_ADDR.to_string()
        ).unwrap();



        // Verify the generated message
        assert!(msg.is_some());

        let expected_sent_coin = Coin::new(
            desired_send_amount.u128(),
            asset.denom.clone()
        );
        matches!(
            msg.unwrap(),
            NativeAssetMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount })
            if to_address == SENDER_ADDR.to_string()
                && amount == vec![expected_sent_coin]
        );

    }


    #[test]
    fn test_send_asset_zero_amount() {

        let env = mock_env();

        let asset = get_mock_asset();
        let desired_send_amount = Uint128::zero();



        // Tested action: send asset
        let msg = asset.send_asset(
            &env,
            desired_send_amount,
            RECEIVER_ADDR.to_string()
        ).unwrap();



        // Verify the generated message
        assert!(msg.is_none());
    }


    // NOTE: 'query_prior_balance' cannot be unit tested, as the operation requires the interaction
    // with the bank module.

}