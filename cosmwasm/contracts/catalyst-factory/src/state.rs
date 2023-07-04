use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr,
    DepsMut, Response, Event, MessageInfo, Deps, Uint128, Uint64};
use cw_storage_plus::Item;

use crate::{error::ContractError, event::{set_default_governance_fee_share_event, set_owner_event}};


// Factory Constants
pub const MAX_GOVERNANCE_FEE_SHARE : Uint64 = Uint64::new(75u64 * 10000000000000000u64);        // 75% 

// Factory storage
pub const OWNER: Item<Addr> = Item::new("catalyst-factory-owner");
pub const DEFAULT_GOVERNANCE_FEE_SHARE: Item<Uint64> = Item::new("catalyst-factory-default-governance-fee");
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
        set_owner_event(account.to_string())
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
) -> Result<Uint64, ContractError> {

    DEFAULT_GOVERNANCE_FEE_SHARE.load(deps.storage).map_err(|err| err.into())

}


pub fn set_default_governance_fee_share_unchecked(
    deps: &mut DepsMut,
    fee: Uint64
) -> Result<Event, ContractError> {

    if fee > MAX_GOVERNANCE_FEE_SHARE {
        return Err(
            ContractError::InvalidDefaultGovernanceFeeShare { requested_fee: fee, max_fee: MAX_GOVERNANCE_FEE_SHARE }
        )
    }

    DEFAULT_GOVERNANCE_FEE_SHARE.save(deps.storage, &fee)?;

    return Ok(
        set_default_governance_fee_share_event(fee)
    )
}


pub fn set_default_governance_fee_share(
    deps: &mut DepsMut,
    info: MessageInfo,
    fee: Uint64
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
    pub vault_code_id: u64,
    pub assets: Vec<String>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<Uint128>,
    pub amplification: Uint64,
    pub chain_interface: Option<String>,
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

pub fn get_deploy_vault_reply_args(
    deps: DepsMut
) -> Result<DeployVaultReplyArgs, ContractError> {
    let args = DEPLOY_VAULT_REPLY_ARGS.load(deps.storage).map_err(|err| err.into());
    DEPLOY_VAULT_REPLY_ARGS.remove(deps.storage);
    args
}