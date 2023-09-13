use cosmwasm_std::{Deps, DepsMut, Uint128, MessageInfo, Env};

use crate::error::VaultTokenError;

pub mod vault_token_native;
pub mod vault_token_cw20;


// Trait defining the interface of the vault token handler struct.
pub trait VaultTokenTrait<Msg> {

    /// Create a new vault token.
    /// 
    /// May return a `Msg` to order the creation of the token.
    /// 
    /// # Arguments:
    /// * `name` - The name of the vault token.
    /// * `symbol` - The symbol of the vault token.
    /// * `decimals` - The decimals of the vault token.
    /// 
    fn create(
        deps: &mut DepsMut,
        env: &Env,
        name: String,
        symbol: String,
        decimals: u8
    ) -> Result<Option<Msg>, VaultTokenError>;


    /// Load the vault token from storage (if necessary).
    fn load(deps: &Deps) -> Result<Self, VaultTokenError> where Self: Sized;


    /// Query the total supply of the vault token.
    /// 
    /// ! **IMPORTANT**: `mint`/`burn` operations may influence the `total_supply` returned
    /// **within** the message execution. The time at which the `mint`/`burn` operations are 
    /// commited is **NOT** guaranteed. To obtain consistent behavior across implementations of
    /// the trait, always cache the `total_supply` using this method before executing any
    /// `mint`/`burn` operations.
    /// 
    fn query_total_supply(&self, deps: &Deps) -> Result<Uint128, VaultTokenError>;


    /// Query an address' vault token balance.
    /// 
    /// ! **IMPORTANT**: `mint`/`burn` operations may influence the `balance` returned
    /// **within** the message execution. The time at which the `mint`/`burn` operations are 
    /// commited is **NOT** guaranteed. To obtain consistent behavior across implementations of
    /// the trait, always cache the `balance` using this method before executing any
    /// `mint`/`burn` operations.
    ///
    fn query_balance(
        &self,
        deps: &Deps,
        address: String
    ) -> Result<Uint128, VaultTokenError>;


    /// Mint a specific amount of vault tokens for a specific recipient.
    /// 
    /// ! **IMPORTANT**: The time at which the `mint` operation is commited is **NOT** guaranteed.
    /// It may vary across implementations of the trait.
    /// 
    /// # Arguments:
    /// * `amount` - The amount of vault tokens to mint.
    /// * `recipient` - The recipient of the minted vault tokens.
    /// 
    fn mint(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128,
        recipient: String
    ) -> Result<Option<Msg>, VaultTokenError>;


    /// Burn a specific amount of vault tokens **from the sender** of the transaction.
    /// 
    /// ! **IMPORTANT**: The time at which the `burn` operation is commited is **NOT** guaranteed.
    /// It may vary across implementations of the trait.
    /// 
    /// # Arguments:
    /// * `amount` - The amount of vault tokens to burn.
    /// 
    fn burn(
        &mut self,
        deps: &mut DepsMut,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128
    ) -> Result<Option<Msg>, VaultTokenError>;
}
