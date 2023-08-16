use cosmwasm_std::{CosmosMsg, DepsMut, Env, Binary};

use crate::error::ContractError;


/// Commands Encoding *****************************************************************************

pub const COMMAND_ALLOW_REVERT_FLAG     : u8 = 0x80;
pub const COMMAND_ID_MASK               : u8 = 0x3f;

//TODO define commands here


/// Get the command id given a raw command byte.
/// 
/// # Arguments:
/// * `raw_command` - The raw command byte.
/// 
#[inline(always)]
pub fn get_command_id(raw_command: u8) -> u8 {
    raw_command & COMMAND_ID_MASK
}

/// Get the 'allow revert' flag given a raw command byte.
/// 
/// # Arguments:
/// * `raw_command` - The raw command byte.
/// 
#[inline(always)]
pub fn get_command_allow_revert_flag(raw_command: u8) -> bool {
    (raw_command & COMMAND_ALLOW_REVERT_FLAG) != 0u8
}




// Commands Execution *****************************************************************************

/// Return type for the commands execution handlers. It can be either a `CosmosMsg` to be
/// dispatched, or the 'Result' of an atomic check operation.
pub enum CommandResult {
    Message(CosmosMsg),
    Check(Result<(), String>)
}

/// Command executor selector.
/// 
/// # Arguments:
/// * `command_id` - The id of the command to be executed.
/// * `input` - The input for the command to be executed.
/// 
pub fn execute_command(
    deps: &mut DepsMut,
    env: &Env,
    command_id: u8,
    input: &Binary
) -> Result<CommandResult, ContractError> {

    match command_id {
        //TODO implement commands here
        _ => unimplemented!()
    }

}
