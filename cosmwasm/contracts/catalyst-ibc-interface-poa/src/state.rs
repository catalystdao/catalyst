use catalyst_ibc_interface::ContractError;
use cosmwasm_std::{Addr, Deps, DepsMut, Event, Response, MessageInfo};
use cw_storage_plus::Item;

pub const OWNER: Item<Addr> = Item::new("catalyst-interface-authority");

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

    // Verify the caller of the transaction is the current owner
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
