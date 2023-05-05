use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr,
    DepsMut, Response, Event, MessageInfo, Deps, Uint128};
use cw_storage_plus::Item;

use crate::error::ContractError;


// Factory Constants
pub const MAX_GOVERNANCE_FEE_SHARE : u64 = 75u64 * 10000000000000000u64;        // 75% 

// Factory storage
pub const OWNER: Item<Addr> = Item::new("catalyst-factory-owner");
pub const DEFAULT_GOVERNANCE_FEE_SHARE: Item<u64> = Item::new("catalyst-factory-default-governance-fee");
pub const DEPLOY_VAULT_REPLY_ARGS: Item<DeployVaultReplyArgs> = Item::new("catalyst-vault-factory-deploy-reply-args");


// Contract owner helpers

//TODO return 'Addr' instead of 'Option<Addr>'? Or keep the 'option' in case 'renounce_ownership' is implemented in the future?
pub fn owner(
    deps: Deps
) -> Result<Option<Addr>, ContractError> {
    OWNER.may_load(deps.storage).map_err(|err| err.into())
}

pub fn is_owner(
    deps: Deps,
    account: Addr,
) -> Result<bool, ContractError> {

    let owner = OWNER.may_load(deps.storage)?;

    match owner {
        Some(saved_value) => Ok(saved_value == account),
        None => Ok(false)
    }

}

pub fn set_owner_unchecked(
    deps: &mut DepsMut,
    account: Addr
) -> Result<Event, ContractError> {
    OWNER.save(deps.storage, &account)?;
    
    Ok(
        Event::new(String::from("SetOwner"))
            .add_attribute("owner", account)
    )
}

pub fn set_owner(
    deps: &mut DepsMut,
    info: MessageInfo,
    account: String
) -> Result<Response, ContractError> {

    // Verify the caller of the transaction is the current factory owner
    if !is_owner(deps.as_ref(), info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Validate the new owner account
    let account = deps.api.addr_validate(account.as_str())?;

    // Set the new owner
    let set_owner_event = set_owner_unchecked(deps, account)?;     //TODO overhaul event

    Ok(
        Response::new()
            .add_event(set_owner_event)
    )

}


// Default governance fee share helpers
pub fn default_governance_fee_share(
    deps: Deps
) -> Result<u64, ContractError> {

    DEFAULT_GOVERNANCE_FEE_SHARE.load(deps.storage).map_err(|err| err.into())

}


pub fn set_default_governance_fee_share_unchecked(
    deps: &mut DepsMut,
    fee: u64
) -> Result<Event, ContractError> {

    if fee > MAX_GOVERNANCE_FEE_SHARE {
        return Err(
            ContractError::InvalidDefaultGovernanceFeeShare { requested_fee: fee, max_fee: MAX_GOVERNANCE_FEE_SHARE }
        )
    }

    DEFAULT_GOVERNANCE_FEE_SHARE.save(deps.storage, &fee)?;

    return Ok(
        Event::new(String::from("SetDefaultGovernanceFeeShare"))
            .add_attribute("fee", fee.to_string())
    )
}


pub fn set_default_governance_fee_share(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: u64
) -> Result<Response, ContractError> {

    // Verify the caller of the transaction is the factory owner
    if !is_owner(deps.as_ref(), info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    // Set the new default governance fee
    let event = set_default_governance_fee_share_unchecked(deps, fee)?;     //TODO overhaul event

    Ok(Response::new().add_event(event))
}


// Deply vault reply helpers

pub const DEPLOY_VAULT_REPLY_ID: u64 = 0x100;

#[cw_serde]
pub struct DeployVaultReplyArgs {
    pub assets: Vec<String>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<u64>,
    pub amplification: u64,
    pub depositor: Addr
}

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

pub fn load_deploy_vault_reply_args(
    deps: Deps
) -> Result<DeployVaultReplyArgs, ContractError> {
    DEPLOY_VAULT_REPLY_ARGS.load(deps.storage).map_err(|err| err.into())
}