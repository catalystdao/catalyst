use cosmwasm_std::{Deps, DepsMut, Binary};
use cw_storage_plus::Map;

use crate::{commands::CommandResult, error::ContractError};

pub const CANCEL_ORDERS: Map<(&str, &str), bool> = Map::new("catalyst-router-cancel-orders");


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
    deps: &Deps,
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
    deps: &Deps,
    authority: String,
    identifier: Binary
) -> Result<CommandResult, ContractError> {

    let cancel_swap = get_cancel_swap_state(
        deps,
        authority.clone(),
        identifier.clone()
    )?;

    if cancel_swap {
        Ok(CommandResult::Check(Err(
            format!(
                "Swap cancelled (authority {}, identifier {})",
                authority,
                identifier.to_base64()
            )
        )))
    }
    else {
        Ok(CommandResult::Check(Ok(())))
    }
    
}



#[cfg(test)]
mod cancel_swap_test {
    use cosmwasm_std::{testing::mock_dependencies, Binary};

    use crate::executors::cancel_swap::{set_cancel_swap_state, get_cancel_swap_state};
    

    #[test]
    fn test_set_and_get_cancel_state() {

        let mut deps = mock_dependencies();

        let authority = "authority";
        let identifier = Binary("id".as_bytes().to_vec());



        // Tested action 1: set state
        set_cancel_swap_state(
            &mut deps.as_mut(),
            authority.to_string(),
            identifier.clone(),
            true
        ).unwrap();



        // Tested action 2: get state
        let state = get_cancel_swap_state(
            &deps.as_ref(),
            authority.to_string(),
            identifier.clone()
        ).unwrap();



        // Make sure 'state' is set
        assert!(state)

    }


    #[test]
    fn test_get_cancel_state_unset() {

        let deps = mock_dependencies();

        let authority = "authority";
        let identifier = Binary("id".as_bytes().to_vec());



        // Tested action: get state without setting it beforehand
        let state = get_cancel_swap_state(
            &deps.as_ref(),
            authority.to_string(),
            identifier.clone()
        ).unwrap();



        // Make sure 'state' is not set
        assert!(!state)

    }


    #[test]
    fn test_reset_cancel_state() {

        let mut deps = mock_dependencies();

        let authority = "authority";
        let identifier = Binary("id".as_bytes().to_vec());

        // Set cancel state
        set_cancel_swap_state(
            &mut deps.as_mut(),
            authority.to_string(),
            identifier.clone(),
            true
        ).unwrap();



        // Tested action: unset the cancel state
        set_cancel_swap_state(
            &mut deps.as_mut(),
            authority.to_string(),
            identifier.clone(),
            false
        ).unwrap();



        // Make sure 'state' is not set
        let state = get_cancel_swap_state(
            &deps.as_ref(),
            authority.to_string(),
            identifier.clone()
        ).unwrap();

        assert!(!state)

    }
}
