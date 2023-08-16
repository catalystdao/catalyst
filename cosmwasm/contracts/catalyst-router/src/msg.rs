use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Uint128};


#[cw_serde]
pub struct InstantiateMsg {
}


#[cw_serde]
pub enum ExecuteMsg {

    Execute {
        commands: Binary,
        inputs: Vec<Binary>,
        deadline: Option<u64>
    },

    OnCatalystCall {
        purchased_tokens: Uint128,
        data: Binary
    }

}


// Reply id flags/masks and helpers

pub const REPLY_ID_FLAG_IS_LAST      : u64 = 0x8000000000000000;
pub const REPLY_ID_FLAG_ALLOW_REVERT : u64 = 0x4000000000000000;
pub const REPLY_ID_INDEX_MASK        : u64 = 0x3fffffffffffffff;


/// Get the 'is last' flag within a reply id.
/// 
/// # Arguments:
/// * `id` - The reply id.
/// 
#[inline(always)]
pub fn get_reply_is_last_flag(id: u64) -> bool {
    (id & REPLY_ID_FLAG_IS_LAST) != 0
}


/// Get the 'allow revert' flag within a reply id.
/// 
/// # Arguments:
/// * `id` - The reply id.
/// 
#[inline(always)]
pub fn get_reply_allow_revert_flag(id: u64) -> bool {
    (id & REPLY_ID_FLAG_ALLOW_REVERT) != 0
}


/// Get the command index within a reply id.
/// 
/// # Arguments:
/// * `id` - The reply id.
/// 
#[inline(always)]
pub fn get_reply_command_index(id: u64) -> usize {
    (id & REPLY_ID_INDEX_MASK) as usize
}
