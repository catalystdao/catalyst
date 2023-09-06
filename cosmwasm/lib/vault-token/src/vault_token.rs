use cosmwasm_std::{Deps, DepsMut, Uint128, MessageInfo, Env};

use crate::error::VaultTokenError;

pub mod vault_token_native;
pub mod vault_token_cw20;

pub trait VaultTokenTrait<Msg> {

    // TODO get vault token info?

    fn create(
        deps: &mut DepsMut,
        env: &Env,
        name: String,
        symbol: String,
        decimals: u8
    ) -> Result<Option<Msg>, VaultTokenError>;

    fn load(deps: &Deps) -> Result<Self, VaultTokenError> where Self: Sized;

    fn query_total_supply(&self, deps: &Deps) -> Result<Uint128, VaultTokenError>;

    fn mint(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<Msg>, VaultTokenError>;

    fn burn(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<Msg>, VaultTokenError>;
}