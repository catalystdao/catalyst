use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Deps, DepsMut, Env, MessageInfo};
use cw20_base::{state::{TOKEN_INFO, TokenInfo, MinterData}, contract::{execute_mint, execute_burn}};

use crate::error::VaultTokenError;
use super::VaultTokenTrait;


// NOTE: See the `VaultTokenTrait` definition for documentation on the implemented methods.


#[cw_serde]
pub enum Cw20VaultTokenMsg {
}


// Cw20 vault token handler
pub struct Cw20VaultToken();

impl VaultTokenTrait<Cw20VaultTokenMsg> for Cw20VaultToken {

    fn create(
        deps: &mut DepsMut,
        env: &Env,
        name: String,
        symbol: String,
        decimals: u8
    ) -> Result<Option<Cw20VaultTokenMsg>, VaultTokenError> {
        
        // Store token info using the cw20-base format
        let data = TokenInfo {
            name,
            symbol,
            decimals,
            total_supply: Uint128::zero(),
            mint: Some(MinterData {
                minter: env.contract.address.clone(),  // Set self as minter
                cap: None
            })
        };
    
        TOKEN_INFO.save(deps.storage, &data)?;

        Ok(None)
    }


    fn load(_deps: &Deps) -> Result<Self, VaultTokenError> where Self: Sized {
        Ok(
            Cw20VaultToken()
        )
    }


    fn query_total_supply(&self, deps: &Deps) -> Result<Uint128, VaultTokenError> {
        Ok(
            TOKEN_INFO.load(deps.storage)?.total_supply
        )
    }


    fn mint(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        _info: &MessageInfo,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<Cw20VaultTokenMsg>, VaultTokenError> {

        if amount.is_zero() {
            return Ok(None);
        }

        let mint_result = execute_mint(
            deps.branch(),
            env.clone(),
            MessageInfo {
                sender: env.contract.address.clone(),   // This contract itself is the one 'sending' the mint operation
                funds: vec![],
            },
            recipient,  // NOTE: the address is validated by the 'execute_mint' call
            amount
        );
        
        match mint_result {
            Ok(_) => Ok(None),
            Err(err) => Err(VaultTokenError::MintFailed { reason: err.to_string() }),
        }
        
    }


    fn burn(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<Cw20VaultTokenMsg>, VaultTokenError> {

        if amount.is_zero() {
            return Ok(None);
        }

        let mint_result = execute_burn(
            deps.branch(),
            env.clone(),
            info.clone(),
            amount
        );
        
        match mint_result {
            Ok(_) => Ok(None),
            Err(err) => Err(VaultTokenError::BurnFailed { reason: err.to_string() }),
        }
    }
}



#[cfg(test)]
mod vault_token_cw20_tests{
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Uint128, DepsMut, Env, Addr};
    use cw20_base::{state::TOKEN_INFO, contract::query_balance};

    use crate::{vault_token::VaultTokenTrait, error::VaultTokenError};
    use super::Cw20VaultToken;


    const VAULT_TOKEN_NAME     : &str = "vault_token_name";
    const VAULT_TOKEN_SYMBOL   : &str = "vault_token_symbol";
    const VAULT_TOKEN_DECIMALS : u8  = 5;

    const TOKEN_HOLDER         : &str = "token_holder";


    fn mock_vault_token(deps: &mut DepsMut, env: &Env) -> Cw20VaultToken {

        Cw20VaultToken::create(
            deps,
            env,
            VAULT_TOKEN_NAME.to_string(),
            VAULT_TOKEN_SYMBOL.to_string(),
            VAULT_TOKEN_DECIMALS
        ).unwrap();

        Cw20VaultToken::load(&deps.as_ref()).unwrap()
    }

    fn fund_test_account(deps: &mut DepsMut, env: &Env, amount: Uint128) {

        let mut vault_token = Cw20VaultToken::load(&deps.as_ref()).unwrap();

        vault_token.mint(
            deps,
            &env,
            &mock_info(
                &"", // The 'sender' of the transaction doesn't matter
                &[]
            ),
            amount,
            TOKEN_HOLDER.to_string()
        ).unwrap();
    }


    #[test]
    fn test_vault_token_creation() {

        let mut deps = mock_dependencies();
        let env = mock_env();



        // Tested action: create vault token
        let result = Cw20VaultToken::create(
            &mut deps.as_mut(),
            &env,
            VAULT_TOKEN_NAME.to_string(),
            VAULT_TOKEN_SYMBOL.to_string(),
            VAULT_TOKEN_DECIMALS
        ).unwrap();     // Make sure the transaction passes



        // Make sure no msg is returned (token info is saved to the 'contract' itself)
        assert!(result.is_none());

        // Verify the saved token info
        let token_info = TOKEN_INFO.load(deps.as_ref().storage).unwrap();
        assert_eq!(
            token_info.name,
            VAULT_TOKEN_NAME.to_string()
        );
        assert_eq!(
            token_info.symbol,
            VAULT_TOKEN_SYMBOL.to_string()
        );
        assert_eq!(
            token_info.decimals,
            VAULT_TOKEN_DECIMALS
        );

        // Verify the total supply
        let total_supply = Cw20VaultToken::load(&deps.as_ref()).unwrap()
            .query_total_supply(&deps.as_ref()).unwrap();
        assert_eq!(
            total_supply,
            Uint128::zero()
        );

    }


    // No need to test the 'load' method, as it does nothing on the `cw20` implementation.

    
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



        // Check no messages are generated
        assert!(result.is_none());

        // Check the user's balance
        let balance = query_balance(
            deps.as_ref(),
            TOKEN_HOLDER.to_string()
        ).unwrap().balance;
        assert_eq!(
            balance,
            mint_amount
        );

        // Verify the total supply
        let total_supply = Cw20VaultToken::load(&deps.as_ref()).unwrap()
            .query_total_supply(&deps.as_ref()).unwrap();
        assert_eq!(
            total_supply,
            mint_amount
        );

    }

    
    #[test]
    fn test_mint_unauthorized() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let mint_amount = Uint128::from(678u128);



        // Tested action: unauthorized token mint
        let mut unauthorized_env = env.clone();
        unauthorized_env.contract.address = Addr::unchecked("unauthorized-account");

        let result = vault_token.mint(
            &mut deps.as_mut(),
            &unauthorized_env,
            &mock_info(
                &"", // The 'sender' of the transaction doesn't matter
                &[]
            ),
            mint_amount,
            TOKEN_HOLDER.to_string()
        );



        // Make sure the transaction fails
        assert!(matches!(
            result.err().unwrap(),
            VaultTokenError::MintFailed { reason }
                if reason == "Unauthorized".to_string()
        ));

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
        ).unwrap(); // Make sure the transaction succeeds



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

        let fund_amount = Uint128::from(1000u128);
        fund_test_account(
            &mut deps.as_mut(),
            &env,
            fund_amount
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
            burn_amount,
        ).unwrap();



        // Check no messages are generated
        assert!(result.is_none());

        // Check the user's balance
        let balance = query_balance(
            deps.as_ref(),
            TOKEN_HOLDER.to_string()
        ).unwrap().balance;
        assert_eq!(
            balance,
            fund_amount - burn_amount
        );

        // Verify the total supply
        let total_supply = Cw20VaultToken::load(&deps.as_ref()).unwrap()
            .query_total_supply(&deps.as_ref()).unwrap();
        assert_eq!(
            total_supply,
            fund_amount - burn_amount
        );

    }

    
    #[test]
    fn test_burn_zero() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let burn_amount = Uint128::zero();



        // Tested action: burn tokens
        let result = vault_token.burn(
            &mut deps.as_mut(),
            &env,
            &mock_info(
                TOKEN_HOLDER,
                &[]
            ),
            burn_amount,
        ).unwrap(); // Make sure the transaction succeeds



        // Check no messages are generated
        assert!(result.is_none());

    }

    
    #[test]
    fn test_burn_without_balance() {

        let mut deps = mock_dependencies();
        let env = mock_env();

        let mut vault_token = mock_vault_token(
            &mut deps.as_mut(),
            &env
        );

        let fund_amount = Uint128::from(1000u128);
        fund_test_account(
            &mut deps.as_mut(),
            &env,
            fund_amount
        );

        let burn_amount = fund_amount + Uint128::one(); // Burn amount larger than account's balance



        // Tested action: burn tokens
        let result = vault_token.burn(
            &mut deps.as_mut(),
            &env,
            &mock_info(
                TOKEN_HOLDER,
                &[]
            ),
            burn_amount,
        );


        // Make sure the transaction fails
        assert!(matches!(
            result.err().unwrap(),
            VaultTokenError::BurnFailed { reason }
                if reason == format!(
                    "Overflow: Cannot Sub with {} and {}",
                    fund_amount,
                    burn_amount
                )
        ))

    }
}
