use catalyst_ibc_interface::ContractError;
use cosmwasm_std::{Addr, Deps, DepsMut, Event, Response, MessageInfo, Empty};
use cw_controllers::Admin;

const ADMIN: Admin = Admin::new("catalyst-factory-admin");

pub fn owner(
    deps: Deps
) -> Result<Option<Addr>, ContractError> {

    ADMIN.get(deps)
        .map_err(|err| err.into())

}

pub fn is_owner(
    deps: Deps,
    account: Addr,
) -> Result<bool, ContractError> {

    ADMIN.is_admin(deps, &account)
        .map_err(|err| err.into())

}

pub fn set_owner_unchecked(
    deps: DepsMut,
    account: Addr
) -> Result<Event, ContractError> {
    
    ADMIN.set(deps, Some(account.clone()))?;
    
    Ok(
        set_owner_event(account.to_string())
    )
}

pub fn update_owner<T>(
    deps: DepsMut,
    info: MessageInfo,
    account: String
) -> Result<Response<T>, ContractError> {

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
        Response::new()
            .add_event(set_owner_event(account.to_string()))
    )

}

pub fn set_owner_event(
    account: String
) -> Event {
    Event::new("set-owner")
        .add_attribute("account", account)
}
