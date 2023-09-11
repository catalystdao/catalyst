use cosmwasm_std::{CosmosMsg, DepsMut, Env, Binary};

use crate::{error::ContractError, executors::{catalyst::catalyst_executors, payments::payments_executors, cancel_swap::cancel_swap_executors}};


/// Commands Encoding *****************************************************************************

// TODO do we want to encode the commands as bytes, or use an 'enum' instead?
// TODO   - how would the 'allow revert' flag be encoded then? As another param?

// TODO review:
// NOTE command values have been changed from the EVM implementation as some commands
// have been removed and to accomodate for possible new ones.

pub const COMMAND_ALLOW_REVERT_FLAG     : u8 = 0x80;
pub const COMMAND_ID_MASK               : u8 = 0x3f;

pub const COMMAND_LOCAL_SWAP            : u8 = 0x00;
pub const COMMAND_SEND_ASSET            : u8 = 0x01;
pub const COMMAND_SEND_LIQUIDITY        : u8 = 0x02;
pub const COMMAND_WITHDRAW_EQUAL        : u8 = 0x03;
pub const COMMAND_WITHDRAW_MIXED        : u8 = 0x04;
pub const COMMAND_DEPOSIT_MIXED         : u8 = 0x05;

pub const COMMAND_SWEEP                 : u8 = 0x06;
pub const COMMAND_TRANSFER              : u8 = 0x07;
pub const COMMAND_PAY_PORTION           : u8 = 0x08;
pub const COMMAND_BALANCE_CHECK         : u8 = 0x09;

pub const COMMAND_ALLOW_CANCEL          : u8 = 0x0d;


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

        COMMAND_LOCAL_SWAP     => catalyst_executors::execute_local_swap(&deps.as_ref(), env, input),
        COMMAND_SEND_ASSET     => catalyst_executors::execute_send_asset(&deps.as_ref(), env, input),
        COMMAND_SEND_LIQUIDITY => catalyst_executors::execute_send_liquidity(&deps.as_ref(), env, input),
        COMMAND_WITHDRAW_EQUAL => catalyst_executors::execute_withdraw_equal(&deps.as_ref(), env, input),
        COMMAND_WITHDRAW_MIXED => catalyst_executors::execute_withdraw_mixed(&deps.as_ref(), env, input),
        COMMAND_DEPOSIT_MIXED  => catalyst_executors::execute_deposit_mixed(&deps.as_ref(), env, input),

        COMMAND_SWEEP          => payments_executors::execute_sweep(&deps.as_ref(), env, input),
        COMMAND_TRANSFER       => payments_executors::execute_transfer(&deps.as_ref(), env, input),
        COMMAND_PAY_PORTION    => payments_executors::execute_pay_portion(&deps.as_ref(), env, input),
        COMMAND_BALANCE_CHECK  => payments_executors::execute_balance_check(&deps.as_ref(), env, input),

        COMMAND_ALLOW_CANCEL   => cancel_swap_executors::execute_cancel_swap(deps, env, input),

        _ => Err(ContractError::InvalidCommand{command_id})
    }

}
