use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Deps, DepsMut, Env, MessageInfo, Int128};
use cw20_base::{state::{TOKEN_INFO, TokenInfo, MinterData}, contract::{execute_mint, execute_burn}};

use crate::error::VaultTokenError;

use super::VaultTokenTrait;




#[cw_serde]
pub enum Cw20VaultTokenMsg {
}



//TODO what about events emitted by execute_mint and execute_burn?

pub struct Cw20VaultToken{
    supply_offset: Int128   //TODO remove and add warning?
}

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
            Cw20VaultToken {
                supply_offset: Int128::zero()
            }
        )
    }

    fn query_prior_total_supply(&self, deps: &Deps) -> Result<Uint128, VaultTokenError> {

        let supply = TOKEN_INFO.load(deps.storage)?.total_supply;

        let offset = Uint128::from(self.supply_offset.i128() as u128);

        Ok(
            supply.wrapping_sub(offset) //TODO review 'wrapping_sub'
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

        self.supply_offset = self.supply_offset.checked_add(
            Int128::from(
                TryInto::<i128>::try_into(amount.u128()).unwrap()   //TODO remove unwrap()
            )
        )?;

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

        self.supply_offset = self.supply_offset.checked_sub(
            Int128::from(
                TryInto::<i128>::try_into(amount.u128()).unwrap()   //TODO remove unwrap()
            )
        )?;

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