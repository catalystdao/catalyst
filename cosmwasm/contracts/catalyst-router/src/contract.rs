#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Reply, StdResult, Deps, Binary, Empty, from_json};
use cw2::set_contract_version;

use crate::commands::CommandOrder;
use crate::dispatcher::{start_dispatching, resume_dispatching};
use crate::error::ContractError;
use crate::executors::cancel_swap::set_cancel_swap_state;
use crate::msg::{ExecuteMsg, InstantiateMsg, get_reply_allow_revert_flag, get_reply_command_index, get_reply_is_last_flag, ExecuteParams};
use crate::state::{lock_router, unlock_router};

// Version information
const CONTRACT_NAME: &str = "catalyst-router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



// Instantiation **********************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}



// Execution **************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    match msg {

        ExecuteMsg::Execute {
            command_orders,
            deadline
        } => execute_execute(
            &mut deps,
            &env,
            info,
            command_orders,
            deadline
        ),

        // `ExecuteMsg` can be extended with a `WrapExecute` variant to `wrap` a router command
        // that requires the execution of multiple `CosmosMsg` (e.g. to transfer multiple `cw20`
        // tokens). This would be required because of the `allow_revert` feature of the router, as 
        // if multiple `CosmosMsg` are emitted and one of them fails (that is not the first one),
        // the router would not be able to revert the state of the messages that did not fail. If
        // all the messages are wrapped within one 'main' message, any failure would cause the
        // 'main' message to revert.

        ExecuteMsg::OnCatalystCall {
            purchased_tokens: _,
            data
        } => {
            let params: ExecuteParams = from_json(&data)?;

            execute_execute(
                &mut deps,
                &env,
                info,
                params.command_orders,
                params.deadline
            )
        },

        ExecuteMsg::CancelSwap {
            identifier,
            state
        } => execute_cancel_swap(
            &mut deps,
            info,
            identifier,
            state
        )
        
    }

}


/// Start the execution of a router instruction (may involve multiple commands/inputs).
/// 
/// **NOTE**: This function may only be invoked if there is no other router instruction execution
/// ongoing.
/// 
/// # Arguments:
/// * `command_orders` - The router command orders to execute.
/// * `deadline` - Time at which the router request expires.
/// 
fn execute_execute(
    deps: &mut DepsMut,
    env: &Env,
    info: MessageInfo,
    command_orders: Vec<CommandOrder>,
    deadline: Option<u64>
) -> Result<Response, ContractError> {

    if let Some(time) = deadline {
        if env.block.time.seconds() > time {
            return Err(ContractError::TransactionDeadlinePassed {})
        };
    }

    // NOTE: It is important to lock the router here and not only if a message is generated, as some
    // instructions may rely on the lock to be present to query the original sender of the transaction.
    lock_router(deps, info)?;

    let initial_submessage = start_dispatching(
        deps,
        env,
        command_orders
    )?;

    let response = match initial_submessage {
        Some(submessage) => Response::new().add_submessage(submessage),
        None => {

            // If there is no submessage to be processed it means that the execution of the
            // router instruction has completed.
            unlock_router(deps);

            Response::new()
        }
    };

    Ok(response)
}


/// Set a cancel swap order.
/// 
/// # Arguments:
/// * `identifier` - The swap identifier.
/// * `state` - Optional 'cancel' state (None defaults to true, i.e. cancel the swap).
/// 
fn execute_cancel_swap(
    deps: &mut DepsMut,
    info: MessageInfo,
    identifier: Binary,
    state: Option<bool>
) -> Result<Response, ContractError> {

    set_cancel_swap_state(
        deps,
        info.sender.to_string(),
        identifier,
        state.unwrap_or(true)
    )?;

    Ok(Response::new())

}




// Reply ******************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    mut deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<Response, ContractError> {

    if reply.result.is_err() && !get_reply_allow_revert_flag(reply.id) {

        return Err(ContractError::CommandReverted{
            index: get_reply_command_index(reply.id) as u64,
            error: reply.result.unwrap_err()
        });

    }


    let mut response = Response::new();

    let mut is_last = get_reply_is_last_flag(reply.id);

    if !is_last {
        // If the message that was just processed is not the last one, resume dispatching
        // the remaining messages.
    
        match resume_dispatching(
            &mut deps,
            &env,
            get_reply_command_index(reply.id) + 1
        )? {
            Some(submessage) => {
                response = response.add_submessage(submessage);
            },
            None => {
                // If there is no submessage to be processed it means that the execution of the
                // router instruction has completed.
                is_last = true;
            }
        };

    }

    if is_last {
        // If all of the commands have been processed, remove the router lock.
        unlock_router(&mut deps);
    }

    Ok(response)
}




// Query ******************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {

    // The router does not implement any queries.
    Err(ContractError::NoQueries{}.into())

}







#[cfg(test)]
mod catalyst_router_tests {
}