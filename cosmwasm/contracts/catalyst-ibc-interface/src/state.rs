use cosmwasm_schema::cw_serde;
use cosmwasm_std::{IbcEndpoint, Deps, Addr, DepsMut, Event, MessageInfo, Empty, Response, Uint64};
use cw_controllers::Admin;
use cw_storage_plus::{Map, Item};

use crate::{ContractError, event::set_owner_event};

// Interface storage
pub const OPEN_CHANNELS: Map<&str, IbcChannelInfo> = Map::new("catalyst-ibc-interface-open-channels");

const ADMIN: Admin = Admin::new("catalyst-ibc-interface-admin");



// IBC
// ************************************************************************************************

// Use a stripped down version of cosmwasm_std::IbcChannel to store the information of the
// interface's open channels.
#[cw_serde]
pub struct IbcChannelInfo {
    pub endpoint: IbcEndpoint,
    pub counterparty_endpoint: IbcEndpoint,
    pub connection_id: String
}



// Underwriting
// ************************************************************************************************

const MAX_UNDERWRITE_DURATION_ALLOWED_SECONDS: Uint64 = Uint64::new(15 * 24 * 60 * 60); // 15 days

const MAX_UNDERWRITE_DURATION_SECONDS: Item<Uint64> = Item::new("catalyst-ibc-interface-max-underwrite-duration");


/// Set the maximum underwriting duration (only applies to new underwrite orders).
/// 
/// NOTE: This function checks that the sender of the transaction is the current interface owner.
/// 
/// # Arguments:
/// * `new_max_duration` - The new desired maximum underwriting duration.
pub fn set_max_underwriting_duration(
    deps: &mut DepsMut,
    info: &MessageInfo,
    new_max_duration: Uint64
) -> Result<Response, ContractError> {

    only_owner(deps.as_ref(), info)?;

    if new_max_duration > MAX_UNDERWRITE_DURATION_ALLOWED_SECONDS {
        return Err(ContractError::MaxUnderwriteDurationTooLong {
            set_duration: new_max_duration,
            max_duration: MAX_UNDERWRITE_DURATION_ALLOWED_SECONDS,
        })
    }

    MAX_UNDERWRITE_DURATION_SECONDS.save(deps.storage, &new_max_duration)?;

    Ok(Response::new())

}


/// Get the maximum underwriting duration.
pub fn get_max_underwrite_duration(
    deps: &Deps
) -> Result<Uint64, ContractError> {

    MAX_UNDERWRITE_DURATION_SECONDS
        .load(deps.storage)
        .map_err(|err| err.into())

}



// Admin
// ************************************************************************************************

/// Get the current interface owner.
pub fn owner(
    deps: Deps
) -> Result<Option<Addr>, ContractError> {

    ADMIN.get(deps)
        .map_err(|err| err.into())

}

/// Assert that the message sender is the interface owner.
pub fn only_owner(
    deps: Deps,
    info: &MessageInfo
) -> Result<(), ContractError> {
 
    match is_owner(deps, &info.sender)? {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {})
    }
}

/// Check if an address is the interface owner.
/// 
/// Arguments:
/// 
/// * `account` - The address of the account to check whether it is the interface owner.
/// 
pub fn is_owner(
    deps: Deps,
    account: &Addr,
) -> Result<bool, ContractError> {

    ADMIN.is_admin(deps, account)
        .map_err(|err| err.into())

}

/// Set the interface owner.
/// 
/// !IMPORTANT: This function DOES NOT check the sender of the transaction.
/// 
/// # Arguments
/// 
/// * `account` - The new interface owner.
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

/// Update the interface owner.
/// 
/// NOTE: This function checks that the sender of the transaction is the current interface owner.
/// 
/// # Arguments
/// 
/// * `account` - The new interface owner.
/// 
pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    account: String
) -> Result<Response, ContractError> {

    // Validate the new owner account
    let account = deps.api.addr_validate(account.as_str())?;

    // ! The 'update' call also verifies whether the caller of the transaction is the current interface owner
    ADMIN.execute_update_admin::<Empty, Empty>(deps, info, Some(account.clone()))
        .map_err(|err| {
            match err {
                cw_controllers::AdminError::Std(err) => err.into(),
                cw_controllers::AdminError::NotAdmin {} => ContractError::Unauthorized {},
            }
        })?;

    Ok(
        Response::new()
            .add_event(set_owner_event(account.to_string()))
    )

}