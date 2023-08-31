use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Deps, Uint128, MessageInfo, CosmosMsg, Env, to_binary};
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};
use cw_storage_plus::Item;

use crate::{asset::{AssetTrait, VaultAssetsTrait}, error::AssetError};

const ASSETS: Item<Vec<String>> = Item::new("catalyst-vault-cw20-assets");


// NOTE: See the `VaultAssetsTrait` and `AssetTrait` definitions for documentation on the
// implemented methods.


/// Vault cw20 asset handler
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


    fn save_refs(&self, deps: &mut DepsMut) -> Result<(), AssetError> {

        let assets_refs = self.get_assets()
            .iter()
            .map(|asset| {
                asset.get_asset_ref().to_owned()
            })
            .collect();

        ASSETS.save(deps.storage, &assets_refs)
            .map_err(|err| err.into())
    }


    fn receive_assets(
        &self,
        env: &Env,
        info: &MessageInfo,
        amounts: Vec<Uint128>
    ) -> Result<Vec<CosmosMsg>, AssetError> {

        // No native assets are expected when handling cw20 assets.
        if info.funds.len() != 0 {
            return Err(AssetError::AssetSurplusReceived {});
        }
        
        if amounts.len() != self.get_assets().len() {
            return Err(AssetError::InvalidParameters {
                reason: "Invalid 'amounts' count when receiving assets.".to_string()
            })
        }

        // NOTE: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // transfer messages for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
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

        // NOTE: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // transfer messages for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
        let cosmos_messages = self.get_assets()
            .iter()
            .zip(amounts)
            .filter(|(_, amount)| !amount.is_zero())     // Do not create transfer messages for zero-valued transfers
            .map(|(asset, amount)| {
                Ok(CosmosMsg::Wasm(
                    cosmwasm_std::WasmMsg::Execute {
                        contract_addr: asset.0.to_owned(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: recipient.clone(),
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



/// Cw20 asset handler
/// 
/// NOTE: For cw20 assets, the asset *reference* is the same as the cw20 token address.
///  
#[cw_serde]
pub struct Cw20Asset(pub String);

impl AssetTrait for Cw20Asset {

    fn from_asset_ref(_deps: &Deps, asset_ref: &str) -> Result<Self, AssetError> {
        Ok(Cw20Asset(asset_ref.to_owned()))
    }


    fn get_asset_ref(&self) -> &str {
        &self.0
    }


    fn save(&self, _deps: &mut DepsMut) -> Result<(), AssetError> {
        Ok(())
    }


    fn query_prior_balance(
        &self,
        deps: &Deps,
        env: &Env,
        _info: Option<&MessageInfo>
    ) -> Result<Uint128, AssetError> {

        // For cw20 assets, the *prior balance* is the real current balance, as *received* cw20
        // assets are processed at the **end** of the message execution.
        
        let queried_balance = deps.querier.query_wasm_smart::<BalanceResponse>(
            self.0.to_owned(),
            &Cw20QueryMsg::Balance { address: env.contract.address.to_string() }
        )?.balance;

        Ok(queried_balance)
    }


    fn receive_asset(
        &self,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<CosmosMsg>, AssetError> {

        if info.funds.len() != 0 {
            return Err(AssetError::AssetSurplusReceived {});
        }

        // NOTE: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // transfer messages for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
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


    fn send_asset(
        &self,
        _env: &Env,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<CosmosMsg>, AssetError> {

        // NOTE: Some cw20 contracts disallow zero-valued token transfers. Do not generate
        // transfer messages for zero-valued balance transfers to prevent these cases from 
        // resulting in failed transactions.
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



#[cfg(test)]
mod asset_cw20_tests {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Uint128, CosmosMsg, WasmMsg, to_binary};
    use cw20::Cw20ExecuteMsg;

    use crate::{asset::{VaultAssetsTrait, AssetTrait}, error::AssetError};

    use super::{Cw20VaultAssets, Cw20Asset};

    const SENDER_ADDR   : &str = "sender_addr";
    const RECEIVER_ADDR : &str = "receiver_addr";

    
    fn get_mock_assets() -> Vec<Cw20Asset> {
        vec![
            Cw20Asset("contract_a".to_string()),
            Cw20Asset("contract_b".to_string()),
            Cw20Asset("contract_c".to_string())
        ]
    }

    fn verify_cw20_transfer_from_msgs(
        cosmos_msgs: Vec<CosmosMsg>,
        assets: Vec<Cw20Asset>,
        amounts: Vec<Uint128>,
        owner: String,
        recipient: String
    ) {

        assert!(cosmos_msgs.len() == assets.len());
        assert!(cosmos_msgs.len() == amounts.len());

        cosmos_msgs.iter()
            .zip(&assets)
            .zip(&amounts)
            .for_each(|((cosmos_msg, asset), expected_amount)| {

                let expected_execute_msg = Cw20ExecuteMsg::TransferFrom {
                    owner: owner.to_string(),
                    recipient: recipient.to_string(),
                    amount: expected_amount.clone()
                };

                matches!(
                    cosmos_msg.clone(),
                    CosmosMsg::Wasm(
                        WasmMsg::Execute {
                            contract_addr,
                            msg,
                            funds
                        }
                    )
                        if contract_addr == asset.0
                            && msg == to_binary(&expected_execute_msg).unwrap()
                            && funds == vec![]
                );
            })
    }

    fn verify_cw20_transfer_msgs(
        cosmos_msgs: Vec<CosmosMsg>,
        assets: Vec<Cw20Asset>,
        amounts: Vec<Uint128>,
        recipient: String
    ) {

        assert!(cosmos_msgs.len() == assets.len());
        assert!(cosmos_msgs.len() == amounts.len());

        cosmos_msgs.iter()
            .zip(&assets)
            .zip(&amounts)
            .for_each(|((cosmos_msg, asset), expected_amount)| {

                let expected_execute_msg = Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount: expected_amount.clone()
                };

                matches!(
                    cosmos_msg.clone(),
                    CosmosMsg::Wasm(
                        WasmMsg::Execute {
                            contract_addr,
                            msg,
                            funds
                        }
                    )
                        if contract_addr == asset.0
                            && msg == to_binary(&expected_execute_msg).unwrap()
                            && funds == vec![]
                );
            })
    }


    #[test]
    fn test_new_vault_assets_handler() {

        let assets = get_mock_assets();
        let handler = Cw20VaultAssets::new(assets.clone());

        assert_eq!(
            handler.get_assets().to_owned(),
            assets
        )
    }


    #[test]
    fn test_save_and_load_vault_assets_handler() {

        let mut deps = mock_dependencies();

        let assets = get_mock_assets();
        let handler = Cw20VaultAssets::new(assets.clone());



        // Tested action 1: save handler
        // NOTE: `save_refs` is tested indirectly via the `save` method.
        handler.save(&mut deps.as_mut()).unwrap();



        // Tested action 2: load references only
        let loaded_refs = Cw20VaultAssets::load_refs(&deps.as_ref()).unwrap();
        assert_eq!(
            loaded_refs,
            assets.iter().map(|asset| asset.get_asset_ref().to_owned()).collect::<Vec<String>>()
        );



        // Tested action 3: load the entire handler
        let loaded_handler = Cw20VaultAssets::load(&deps.as_ref()).unwrap();

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
        let handler = Cw20VaultAssets::new(assets.clone());

        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128),
            Uint128::from(789u128)
        ];



        // Tested action: receive assets
        let cosmos_msgs = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]
            ),
            desired_received_amounts.clone()
        ).unwrap();     // Call is successful



        // Verify transfer Cosmos messages are generated
        assert!(cosmos_msgs.len() == 3);
        verify_cw20_transfer_from_msgs(
            cosmos_msgs,
            assets.clone(),
            desired_received_amounts.clone(),
            SENDER_ADDR.to_string(),
            env.contract.address.to_string()
        );

    }


    #[test]
    fn test_handler_receive_asset_invalid_amounts_count() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = Cw20VaultAssets::new(assets.clone());

        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128)      // One amount less than assets hold by the vault
        ];



        // Tested action: receive assets with invalid 'amounts' count
        let result = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]
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
    fn test_handler_receive_assets_zero_amount() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = Cw20VaultAssets::new(assets.clone());



        // Tested action 1: one asset with zero amount
        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::zero(),        // Zero amount
            Uint128::from(789u128)
        ];

        let cosmos_msgs = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]
            ),
            desired_received_amounts.clone()
        ).unwrap();     // Make sure result is successful

        // Verify no transfer msg is generated for the zero-valued asset transfer
        assert!(cosmos_msgs.len() == 2);
        
        verify_cw20_transfer_from_msgs(
            cosmos_msgs,
            vec![assets[0].clone(), assets[2].clone()],             // Skip zero-valued asset
            vec![Uint128::from(123u128), Uint128::from(789u128)],   // Skip zero-valued asset
            SENDER_ADDR.to_string(),
            env.contract.address.to_string()
        );



        // Tested action 2: all assets with zero amount
        let desired_received_amounts: Vec<Uint128> = vec![
            Uint128::zero(),
            Uint128::zero(),
            Uint128::zero()
        ];

        let cosmos_msgs = handler.receive_assets(
            &env,
            &mock_info(
                SENDER_ADDR,
                &[]
            ),
            desired_received_amounts.clone()
        ).unwrap();     // Make sure result is successful

        // Verify no cosmos msgs are generated
        assert!(cosmos_msgs.len() == 0);

    }


    #[test]
    fn test_handler_send_assets() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = Cw20VaultAssets::new(assets.clone());

        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::from(456u128),
            Uint128::from(789u128)
        ];



        // Tested action: send assets
        let cosmos_msgs = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        ).unwrap();



        // Verify that the generated Cosmos messages are valid
        assert!(cosmos_msgs.len() == 3);
        verify_cw20_transfer_msgs(
            cosmos_msgs,
            assets,
            desired_send_amounts,
            RECEIVER_ADDR.to_string()
        );

    }


    #[test]
    fn test_handler_send_assets_invalid_amounts_count() {

        let env = mock_env();

        let assets = get_mock_assets();
        let handler = Cw20VaultAssets::new(assets.clone());

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



        // Verify that the generated Cosmos messages are valid
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
        let handler = Cw20VaultAssets::new(assets.clone());



        // Tested action 1: one asset with zero amount
        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::from(123u128),
            Uint128::zero(),        // Zero amount
            Uint128::from(789u128)
        ];

        let cosmos_msgs = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        ).unwrap();     // Make sure result is successful

        // Verify that no transfer message is generated for the zero-valued asset transfer
        assert!(cosmos_msgs.len() == 2);
        verify_cw20_transfer_msgs(
            cosmos_msgs,
            vec![assets[0].clone(), assets[2].clone()],             // Skip zero-valued asset
            vec![Uint128::from(123u128), Uint128::from(789u128)],   // Skip zero-valued asset
            RECEIVER_ADDR.to_string()
        );



        // Tested action 2: all assets with zero amount
        let desired_send_amounts: Vec<Uint128> = vec![
            Uint128::zero(),
            Uint128::zero(),
            Uint128::zero()
        ];

        let cosmos_msgs = handler.send_assets(
            &env,
            desired_send_amounts.clone(),
            RECEIVER_ADDR.to_string()
        ).unwrap();     // Make sure result is successful

        // Verify that no Cosmos messages are generated
        assert!(cosmos_msgs.len() == 0);

    }

}