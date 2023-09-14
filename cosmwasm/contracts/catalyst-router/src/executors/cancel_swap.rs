use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Binary, from_binary};
use cw_storage_plus::Map;

use crate::{commands::CommandResult, error::ContractError};

pub const CANCEL_ORDERS: Map<(&str, &str), bool> = Map::new("catalyst-router-cancel-orders");

#[cw_serde]
struct AllowCancelCommand {
    authority: String,
    identifier: Binary
}

pub fn set_cancel_swap_state(
    deps: &mut DepsMut,
    authority: String,
    identifier: Binary,
    state: bool
) -> Result<(), ContractError> {
    
    CANCEL_ORDERS.save(
        deps.storage,
        (authority.as_str(), identifier.to_base64().as_str()),
        &state
    )?;

    Ok(())
}

pub fn get_cancel_swap_state(
    deps: &mut DepsMut,
    authority: String,
    identifier: Binary
) -> Result<bool, ContractError> {
    
    let state = CANCEL_ORDERS.may_load(
        deps.storage,
        (authority.as_str(), identifier.to_base64().as_str())
    )?;

    Ok(state.unwrap_or(false))
}


pub fn execute_allow_cancel(
    deps: &mut DepsMut,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    let args = from_binary::<AllowCancelCommand>(input)?;

    let cancel_swap = get_cancel_swap_state(
        deps,
        args.authority.clone(),
        args.identifier.clone()
    )?;

    if cancel_swap {
        Ok(CommandResult::Check(Err(
            format!(
                "Swap cancelled (authority {}, identifier {})",
                args.authority,
                args.identifier.to_base64()
            )
        )))
    }
    else {
        Ok(CommandResult::Check(Ok(())))
    }
    
}
