use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Deps, DepsMut, Env, MessageInfo};
use cw_storage_plus::Item;
use token_bindings::{TokenMsg, Metadata, DenomUnit};

use crate::error::VaultTokenError;

use super::VaultTokenTrait;

const VAULT_TOKEN_DENOM: Item<String> = Item::new("catalyst-vault-token-denom");




#[cw_serde]
pub enum NativeVaultTokenMsg {
    Token(TokenMsg)
}



pub struct NativeVaultToken(String);

impl VaultTokenTrait<NativeVaultTokenMsg> for NativeVaultToken {

    fn create(
        deps: &mut DepsMut,
        env: &Env,
        name: String,
        symbol: String,
        _decimals: u8
    ) -> Result<Option<NativeVaultTokenMsg>, VaultTokenError> {

        let denom = format!("factory/{}/{}", env.contract.address.to_string(), symbol);

        // Save the vault token denom
        VAULT_TOKEN_DENOM.save(
            deps.storage,
            &denom
        )?;

        let metadata = Metadata {
            description: None,
            denom_units: vec![
                DenomUnit {
                    denom: denom.clone(),
                    exponent: 0,
                    aliases: vec![]
                }
            ],
            base: Some(denom.clone()),
            display: Some(denom),
            name: Some(name),
            symbol: Some(symbol.clone()),
        };

        let create_msg = TokenMsg::CreateDenom {
            subdenom: symbol,
            metadata: Some(metadata)
        };

        Ok(
            Some(NativeVaultTokenMsg::Token(create_msg))
        )
    }

    fn load(deps: &Deps) -> Result<Self, VaultTokenError> where Self: Sized {
        
        let denom = VAULT_TOKEN_DENOM.load(deps.storage)?;

        Ok(NativeVaultToken(denom))
    }

    fn query_total_supply(&self, deps: &Deps) -> Result<Uint128, VaultTokenError> {

        let response = deps.querier.query_supply(self.0.clone())?;

        Ok(response.amount)

    }

    fn mint(
        &mut self,
        _deps: &mut DepsMut,
        _env: &Env,
        _info: &MessageInfo,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<NativeVaultTokenMsg>, VaultTokenError> {

        if amount.is_zero() {
            return Ok(None);
        }

        let mint_msg = TokenMsg::MintTokens {
            denom: self.0.to_owned(),
            amount,
            mint_to_address: recipient
        };

        Ok(
            Some(NativeVaultTokenMsg::Token(mint_msg))
        )
        
    }

    fn burn(
        &mut self,
        _deps: &mut DepsMut,
        _env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<NativeVaultTokenMsg>, VaultTokenError> {

        if amount.is_zero() {
            return Ok(None);
        }

        let burn_msg = TokenMsg::BurnTokens {
            denom: self.0.to_owned(),
            amount,
            burn_from_address: info.sender.to_string()
        };

        Ok(
            Some(NativeVaultTokenMsg::Token(burn_msg))
        )
    }
}