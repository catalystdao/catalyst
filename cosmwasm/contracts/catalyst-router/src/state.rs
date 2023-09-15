use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Env, Deps};
use cw_storage_plus::Item;

use crate::{error::ContractError, commands::CommandOrder};

pub const ROUTER_LOCK: Item<RouterLock> = Item::new("router-lock");
pub const ROUTER_STATE: Item<RouterState> = Item::new("router-state");


/// Struct used to *lock* the router execution from reentry/starting a new execution. It
/// contains the address of the account initializing the router execution.
#[cw_serde]
pub struct RouterLock {
    locked_by: Addr
}


/// Struct used to save the state of the router to be able to resume the execution within the 
/// `reply` handler of the router.
#[cw_serde]
pub struct RouterState {
    pub offset: u8,
    pub command_orders: Vec<CommandOrder>
}


/// *Lock* the router from reentry/starting a new execution whilst another execution
/// is ongoing.
/// 
/// **NOTE**: This will check that a lock is **not** already present.
/// 
pub fn lock_router(
    deps: &mut DepsMut,
    info: MessageInfo
) -> Result<(), ContractError> {

    if ROUTER_LOCK.exists(deps.storage) {
        return Err(ContractError::Unauthorized {});
    }

    ROUTER_LOCK.save(
        deps.storage,
        &RouterLock { locked_by: info.sender }
    )?;

    Ok(())

}


/// *Unlock* the router once the ongoing execution finishes.
pub fn unlock_router(
    deps: &mut DepsMut
) -> () {

    ROUTER_LOCK.remove(deps.storage);

}


/// Query the current router locker.
pub fn get_router_locker(
    deps: &Deps
) -> Result<Addr, ContractError> {
    
    let lock = ROUTER_LOCK.load(deps.storage)?;

    Ok(lock.locked_by)

}


/// Verify that the sender of the current invokation is the router itself.
pub fn only_router(
    env: &Env,
    info: MessageInfo
) -> Result<(), ContractError> {

    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    Ok(())
}



#[cfg(test)]
mod lock_tests {
    use cosmwasm_std::{testing::{mock_dependencies, mock_info, mock_env}, Addr};

    use crate::{state::{unlock_router, get_router_locker}, error::ContractError};

    use super::{lock_router, only_router};

    const LOCKER: &str = "locker";


    #[test]
    fn test_router_lock_and_unlock() {

        let mut deps = mock_dependencies();
        let info = mock_info(LOCKER, &[]);



        // Tested action 1: lock router
        lock_router(&mut deps.as_mut(), info).unwrap();

        // Verify lock state
        assert_eq!(
            get_router_locker(&deps.as_ref()).unwrap(),
            Addr::unchecked(LOCKER)
        );



        // Tested action 2: unlock router
        unlock_router(&mut deps.as_mut());

        // Verify lock state
        assert!(
            get_router_locker(&deps.as_ref()).is_err(),
        );
    }


    #[test]
    fn test_router_lock_twice() {

        let mut deps = mock_dependencies();
        let info = mock_info(LOCKER, &[]);

        // Lock the router
        lock_router(&mut deps.as_mut(), info.clone()).unwrap();



        // Tested action: try to lock again (reentry)
        let result = lock_router(&mut deps.as_mut(), info);

        // Make sure lock fails
        assert!(matches!(
            result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }


    #[test]
    fn test_only_router() {

        let env = mock_env();



        // Tested action 1: Only router true
        let sender = env.contract.address.as_str();
        let info = mock_info(sender, &[]);
        let result = only_router(&env, info);

        assert!(result.is_ok());



        // Tested action 2: Only router false
        let sender = "not-the-contract";
        let info = mock_info(sender, &[]);
        let result = only_router(&env, info);

        assert!(matches!(
            result.err().unwrap(),
            ContractError::Unauthorized {}
        ));

    }

}