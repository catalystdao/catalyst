use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Deps, DepsMut, Env, MessageInfo};
use cw_storage_plus::Item;
use token_bindings::{TokenMsg, Metadata, DenomUnit};

use crate::error::VaultTokenError;
use super::VaultTokenTrait;

const VAULT_TOKEN_DENOM: Item<String> = Item::new("catalyst-vault-token-denom");


// NOTE: See the `VaultTokenTrait` definition for documentation on the implemented methods.


#[cw_serde]
pub enum NativeVaultTokenMsg {
    Token(TokenMsg)
}


// Native vault token handler
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



#[cfg(test)]
mod vault_token_cw20_tests{
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Uint128, DepsMut, Env};
    use token_bindings::TokenMsg;

    use crate::vault_token::{VaultTokenTrait, vault_token_native::NativeVaultTokenMsg};
    use super::NativeVaultToken;


    const VAULT_TOKEN_NAME     : &str = "vault_token_name";
    const VAULT_TOKEN_SYMBOL   : &str = "vault_token_symbol";
    const VAULT_TOKEN_DECIMALS : u8  = 5;

    const TOKEN_HOLDER         : &str = "token_holder";


    fn mock_vault_token(deps: &mut DepsMut, env: &Env) -> NativeVaultToken {

        NativeVaultToken::create(
            deps,
            env,
            VAULT_TOKEN_NAME.to_string(),
            VAULT_TOKEN_SYMBOL.to_string(),
            VAULT_TOKEN_DECIMALS
        ).unwrap();

        NativeVaultToken::load(&deps.as_ref()).unwrap()
    }

    fn mock_vault_token_denom() -> String {
        format!("factory/{}/{}", mock_env().contract.address, VAULT_TOKEN_SYMBOL)
    }


    #[test]
    fn test_vault_token_creation() {

        let mut deps = mock_dependencies();
        let env = mock_env();



        // Tested action: create vault token
        let result = NativeVaultToken::create(
            &mut deps.as_mut(),
            &env,
            VAULT_TOKEN_NAME.to_string(),
            VAULT_TOKEN_SYMBOL.to_string(),
            VAULT_TOKEN_DECIMALS
        ).unwrap();     // Make sure the transaction passes



        // Verify the TokenFactory message
        if let NativeVaultTokenMsg::Token(
            TokenMsg::CreateDenom { subdenom, metadata }
        ) = result.unwrap() {

            assert_eq!(
                subdenom,
                VAULT_TOKEN_SYMBOL.to_string()
            );

            assert_eq!(
                metadata.unwrap().name.unwrap(),
                VAULT_TOKEN_NAME.to_string()
            );
        }
        else {
            panic!("Invalid create message.")
        }

        // Verify load
        let loaded_vault_token = NativeVaultToken::load(&deps.as_ref()).unwrap();
        assert_eq!(
            loaded_vault_token.0,
            mock_vault_token_denom()
        );
    }


    #[test]
    fn test_mint() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let mint_amount = Uint128::from(678u128);



        // Tested action: mint tokens
        let result = vault_token.mint(
            &mut deps.as_mut(),
            &env,
            &mock_info(
                &"", // The 'sender' of the transaction doesn't matter
                &[]
            ),
            mint_amount,
            TOKEN_HOLDER.to_string()
        ).unwrap();



        // Check the generated message
        assert!(matches!(
            result.unwrap(),
            NativeVaultTokenMsg::Token(token_msg)
                if token_msg == TokenMsg::MintTokens {
                    denom: mock_vault_token_denom(),
                    amount: mint_amount,
                    mint_to_address: TOKEN_HOLDER.to_string()
                }
        ))

    }


    #[test]
    fn test_mint_zero() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let mint_amount = Uint128::zero();  // Mint zero amount



        // Tested action: mint zero tokens
        let result = vault_token.mint(
            &mut deps.as_mut(),
            &env,
            &mock_info(
                &"", // The 'sender' of the transaction doesn't matter
                &[]
            ),
            mint_amount,
            TOKEN_HOLDER.to_string()
        ).unwrap();



        // Check no messages are generated
        assert!(result.is_none());

    }


    #[test]
    fn test_burn() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let burn_amount = Uint128::from(678u128);



        // Tested action: burn tokens
        let result = vault_token.burn(
            &mut deps.as_mut(),
            &env,
            &mock_info(
                TOKEN_HOLDER,
                &[]
            ),
            burn_amount
        ).unwrap();



        // Check the generated message
        assert!(matches!(
            result.unwrap(),
            NativeVaultTokenMsg::Token(token_msg)
                if token_msg == TokenMsg::BurnTokens {
                    denom: mock_vault_token_denom(),
                    amount: burn_amount,
                    burn_from_address: TOKEN_HOLDER.to_string()
                }
        ))

    }


    #[test]
    fn test_burn_zero() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let burn_amount = Uint128::zero();  // Burn zero amount



        // Tested action: burn zero tokens
        let result = vault_token.burn(
            &mut deps.as_mut(),
            &env,
            &mock_info(
                TOKEN_HOLDER,
                &[]
            ),
            burn_amount
        ).unwrap();



        // Check no messages are generated
        assert!(result.is_none());

    }
}
