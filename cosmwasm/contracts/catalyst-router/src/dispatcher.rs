use cosmwasm_std::{Binary, CosmosMsg, DepsMut, Env, ReplyOn, SubMsg, StdError};

use crate::{error::ContractError, commands::{CommandResult, execute_command, get_command_allow_revert_flag, get_command_id}, state::{ROUTER_STATE, RouterState}};



/// The `DispatchOrder` struct holds a `CosmosMsg` that is to be executed together with some
/// additional data regarding the command execution.
pub struct DispatchOrder {
    pub message: CosmosMsg,
    pub message_index: usize,
    pub allow_revert: bool,
    pub is_last: bool
}

/// Convert a `DispatchOrder` into a Cosmos `SubMsg`. This will encode the command execution
/// additional data into the `SubMsg` `reply_id`.
impl From<DispatchOrder> for SubMsg {
    fn from(value: DispatchOrder) -> Self {

        let reply_id = value.message_index as u64
            | ((value.allow_revert as u64) << 62)
            | ((value.is_last as u64) << 63);
        
        SubMsg{
            id: reply_id,
            msg: value.message,
            gas_limit: None,
            reply_on: ReplyOn::Always
        }
    }
}


/// Process command instructions. Returns an optional `DispatchOrder` containing the next message
/// to be executed, or `None` if all commands have been processed.
/// 
/// # Arguments:
/// * `commands` - The byte-encoded commands to be executed.
/// * `inputs` - Vector of the byte-encoded inputs that correspond to the commands to be executed.
/// * `next_command_index` - The index of the next command that is to be executed.
/// * `offset` - Adjustment factor of the `next_command_index` to match the `commands` and `inputs`
/// variables.
/// 
fn dispatch_commands(
    deps: &mut DepsMut,
    env: &Env,
    commands: &Binary,
    inputs: &Vec<Binary>,
    next_command_index: usize,
    offset: usize
) -> Result<Option<DispatchOrder>, ContractError> {

    let local_commands_count = commands.len();
    let local_resume_index = next_command_index - offset;

    for local_index in local_resume_index..local_commands_count {

        let raw_command = commands
            .get(local_index)
            .unwrap();  // The command at `local_index` should always exist.

        let input = inputs
            .get(local_index)
            .unwrap();  // The input at `local_index` should always exist.


        match execute_command(
            deps,
            env,
            get_command_id(*raw_command),
            input
        )? {

            CommandResult::Message(cosmos_msg) => {

                // Return the `DispatchOrder` for the current command (if required).

                let allow_revert = get_command_allow_revert_flag(*raw_command);
                let is_last = local_index == local_commands_count - 1;

                return Ok(Some(
                    DispatchOrder{
                        message: cosmos_msg,
                        message_index: local_index + offset,
                        allow_revert,
                        is_last
                    }
                ))
            },

            CommandResult::Check(value) => {
                
                // Verify that no error is returned by the command or that the `allow_revert` flag is set.

                //TODO why would this be ever used? If we don't care whether a query succeeds, why include the command at all?
                if value.is_err() && !get_command_allow_revert_flag(*raw_command) {
                    return Err(ContractError::CommandReverted {
                        index: (local_index + offset) as u64,
                        error: value.err().unwrap()
                    })
                }
            },
        }

    }

    Ok(None)    // i.e. All commands have been processed.
}



/// Start processing and dispatching the commands of a router instruction. This function will
/// execute atomically as many commands as possible until a `CosmosMsg` is to be processed (if
/// required by one of the router commands). In that case it will save the **remaining** 
/// commands/inputs (if any) to the chain's store to be able to resume the execution within the
/// router `reply` handler.
/// 
/// # Arguments:
/// * `commands` - The router commands.
/// * `inputs` - The inputs corresponding to the router commands.
/// 
pub fn start_dispatching(
    deps: &mut DepsMut,
    env: &Env,
    commands: Binary,
    inputs: Vec<Binary>
) -> Result<Option<SubMsg>, ContractError> {

    let dispatch_order = dispatch_commands(
        deps,
        env,
        &commands,
        &inputs,
        0,
        0
    )?;

    match dispatch_order {
        Some(order) => {
            if !order.is_last {
                // If further commands are to be executed, save the commands to the store to be able
                // to resume dispatching of the commands within the router `reply` handler.

                let next_message_index = order.message_index + 1;
    
                ROUTER_STATE.save(
                    deps.storage,
                    &RouterState {
                        offset: next_message_index.try_into()
                            .map_err(|_| StdError::GenericErr {
                                msg: "Failed to save the router state offset (too many commands).".to_string()
                            })?,
                        commands: Binary(commands[next_message_index..].to_vec()),
                        inputs: inputs[next_message_index..].to_vec(),
                    }
                )?;
    
            }
    
            Ok(Some(order.into()))
            
        },
        None => {
            Ok(None)
        },
    }

}


/// Resume processing and dispatching the commands of a router instruction. This function will
/// load from the chain's store and execute atomically as many commands as possible until a 
/// `CosmosMsg` is to be processed (if required by one of the remaining router commands). If 
/// no further commands are to be processed, it will delete the commands/inputs from the chain's
/// store.
/// 
/// # Arguments:
/// * `next_command_index` - The index of the next command to be processed.
/// 
pub fn resume_dispatching(
    deps: &mut DepsMut,
    env: &Env,
    next_command_index: usize
) -> Result<Option<SubMsg>, ContractError> {

    let state = ROUTER_STATE.load(deps.storage)?;

    let dispatch_order_option = dispatch_commands(
        deps,
        env,
        &state.commands,
        &state.inputs,
        next_command_index,
        state.offset as usize
    )?;

    match dispatch_order_option {
        Some(order) => {
            Ok(Some(order.into()))
        },
        None => {
            Ok(None)
        }
    }
}
