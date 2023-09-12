#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Reply, StdResult, Deps, Binary, Empty, from_binary};
use cw2::set_contract_version;

use crate::dispatcher::{start_dispatching, resume_dispatching};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, get_reply_allow_revert_flag, get_reply_command_index, get_reply_is_last_flag, ExecuteParams};
use crate::state::{lock_router, unlock_router, ROUTER_STATE};

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
            commands,
            inputs,
            deadline
        } => execute_execute(
            &mut deps,
            &env,
            info,
            commands,
            inputs,
            deadline
        ),

        ExecuteMsg::OnCatalystCall {
            purchased_tokens: _,
            data
        } => {
            let params: ExecuteParams = from_binary(&data)?;

            execute_execute(
                &mut deps,
                &env,
                info,
                params.commands,
                params.inputs,
                params.deadline
            )
        }

        // TODO Batched command
    }

}


/// Start the execution of a router instruction (may involve multiple commands/inputs).
/// 
/// **NOTE**: This function may only be invoked if there is no other router instruction execution
/// ongoing.
/// 
/// # Arguments:
/// * `commands` - The router commands.
/// * `inputs` - The inputs corresponding to the router commands.
/// * `deadline` - Time at which the router request expires.
/// 
fn execute_execute(
    deps: &mut DepsMut,
    env: &Env,
    info: MessageInfo,
    commands: Binary,
    inputs: Vec<Binary>,
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
        commands,
        inputs
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
        // If all of the commands have been processed, remove the router state.
        ROUTER_STATE.remove(deps.storage);
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