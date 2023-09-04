use catalyst_vault_common::asset::{Asset, VaultResponse};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, Event, MessageInfo, Deps, Uint128, Uint64, Empty};
use cw_controllers::Admin;
use cw_storage_plus::Item;

use crate::{error::ContractError, event::{set_default_governance_fee_share_event, set_owner_event}};


// Factory Constants
pub const MAX_DEFAULT_GOVERNANCE_FEE_SHARE : Uint64 = Uint64::new(75u64 * 10000000000000000u64);        // 75% 

// Factory storage
const ADMIN: Admin = Admin::new("catalyst-factory-admin");
pub const DEFAULT_GOVERNANCE_FEE_SHARE: Item<Uint64> = Item::new("catalyst-factory-default-governance-fee");
pub const DEPLOY_VAULT_REPLY_ARGS: Item<DeployVaultReplyArgs> = Item::new("catalyst-vault-factory-deploy-reply-args");


// Contract owner helpers

/// Get the current factory owner.
pub fn owner(
    deps: Deps
) -> Result<Option<Addr>, ContractError> {

    ADMIN.get(deps)
        .map_err(|err| err.into())

}

/// Check if an address is the factory owner.
/// 
/// Arguments:
/// 
/// * `account` - The address of the account to check whether it is the factory owner.
/// 
pub fn is_owner(
    deps: Deps,
    account: Addr,
) -> Result<bool, ContractError> {

    ADMIN.is_admin(deps, &account)
        .map_err(|err| err.into())

}

/// Set the factory owner.
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments
/// 
/// * `account` - The new factory owner.
/// 
pub fn set_owner_unchecked(
    deps: DepsMut,
    account: Addr
) -> Result<Event, ContractError> {
    
    ADMIN.set(deps, Some(account.clone()))?;
    
    Ok(
        set_owner_event(account.to_string())
    )
}

/// Update the factory owner.
/// 
/// NOTE: This function checks that the sender of the transaction is the current factory owner.
/// 
/// # Arguments
/// 
/// * `account` - The new factory owner.
/// 
pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    account: String
) -> Result<VaultResponse, ContractError> {

    // Validate the new owner account
    let account = deps.api.addr_validate(account.as_str())?;

    // ! The 'update' call also verifies whether the caller of the transaction is the current factory owner
    ADMIN.execute_update_admin::<Empty, Empty>(deps, info, Some(account.clone()))
        .map_err(|err| {
            match err {
                cw_controllers::AdminError::Std(err) => err.into(),
                cw_controllers::AdminError::NotAdmin {} => ContractError::Unauthorized {},
            }
        })?;

    Ok(
        VaultResponse::new()
            .add_event(set_owner_event(account.to_string()))
    )

}


// Default governance fee share helpers

/// Get the current default governance fee share.
pub fn default_governance_fee_share(
    deps: Deps
) -> Result<Uint64, ContractError> {

    DEFAULT_GOVERNANCE_FEE_SHARE.load(deps.storage).map_err(|err| err.into())

}


/// Set a new default governance fee share.
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments
/// 
/// * `fee` - The new default governance fee share (18 decimals).
/// 
pub fn set_default_governance_fee_share_unchecked(
    deps: &mut DepsMut,
    fee: Uint64
) -> Result<Event, ContractError> {

    if fee > MAX_DEFAULT_GOVERNANCE_FEE_SHARE {
        return Err(
            ContractError::InvalidDefaultGovernanceFeeShare { requested_fee: fee, max_fee: MAX_DEFAULT_GOVERNANCE_FEE_SHARE }
        )
    }

    DEFAULT_GOVERNANCE_FEE_SHARE.save(deps.storage, &fee)?;

    return Ok(
        set_default_governance_fee_share_event(fee)
    )
}


/// Set a new default governance fee share.
/// 
/// NOTE: This function checks that the sender of the transaction is the current factory owner.
/// 
/// # Arguments
/// 
/// * `fee` - The new default governance fee share (18 decimals).
/// 
pub fn set_default_governance_fee_share(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: Uint64
) -> Result<VaultResponse, ContractError> {

    // Verify the caller of the transaction is the factory owner
    if !is_owner(deps.as_ref(), info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Set the new default governance fee
    let event = set_default_governance_fee_share_unchecked(deps, fee)?;

    Ok(VaultResponse::new().add_event(event))
}


// Deploy vault reply helpers

pub const DEPLOY_VAULT_REPLY_ID: u64 = 0x100;

#[cw_serde]
pub struct DeployVaultReplyArgs {
    pub vault_code_id: u64,
    pub assets: Vec<Asset>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<Uint128>,
    pub amplification: Uint64,
    pub chain_interface: Option<String>,
    pub depositor: Addr
}

/// Save the vault deployment configuration for it to be accessible by the 'reply' handler after instantiation.
/// 
/// # Arguments
/// 
/// * `args` - The vault deployment arguments.
/// 
pub fn save_deploy_vault_reply_args(
    deps: DepsMut,
    args: DeployVaultReplyArgs
) -> Result<(), ContractError> {

    // ! Only save data if the storage item is empty
    if DEPLOY_VAULT_REPLY_ARGS.may_load(deps.storage)?.is_some() {
        return Err(ContractError::Unauthorized {});
    }

    DEPLOY_VAULT_REPLY_ARGS.save(
        deps.storage,
        &args
    ).map_err(|err| err.into())
}

/// Get the vault deployment configuration.
pub fn get_deploy_vault_reply_args(
    deps: DepsMut
) -> Result<DeployVaultReplyArgs, ContractError> {
    let args = DEPLOY_VAULT_REPLY_ARGS.load(deps.storage).map_err(|err| err.into());
    DEPLOY_VAULT_REPLY_ARGS.remove(deps.storage);
    args
}