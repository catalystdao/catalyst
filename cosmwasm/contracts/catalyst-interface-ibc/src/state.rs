use cosmwasm_schema::cw_serde;
use cosmwasm_std::IbcEndpoint;
use cw_storage_plus::Map;


// Constants
pub const TRANSACTION_TIMEOUT_SECONDS: u64 = 2 * 60 * 60;   // 2 hours
pub const WRAPPED_MESSAGES_REPLY_ID: u64 = 0x999;   //TODO move definition to 'common'?


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