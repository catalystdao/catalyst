use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Env, Binary, Deps};
use cw_storage_plus::Item;

use crate::error::ContractError;

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
    pub commands: Binary,
    pub inputs: Vec<Binary>
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