use cosmwasm_schema::cw_serde;
use cosmwasm_std::{IbcEndpoint, Uint64};
use cw_storage_plus::Map;


// Constants
pub const TRANSACTION_TIMEOUT_SECONDS: u64 = 2 * 60 * 60;   // 2 hours
pub const MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS: Uint64 = Uint64::new(24 * 60 * 60);       // 1 day at 1 block/s
pub const MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(15 * 24 * 60 * 60);  // 15 days at 1 block/s
pub const WRAPPED_MESSAGES_REPLY_ID: u64 = 0x999;


// Storage
pub const OPEN_CHANNELS: Map<&str, IbcChannelInfo> = Map::new("catalyst-interface-ibc-open-channels");


// Use a stripped down version of cosmwasm_std::IbcChannel to store the information of the
// interface's open channels.
#[cw_serde]
pub struct IbcChannelInfo {
    pub endpoint: IbcEndpoint,
    pub counterparty_endpoint: IbcEndpoint,
    pub connection_id: String
}