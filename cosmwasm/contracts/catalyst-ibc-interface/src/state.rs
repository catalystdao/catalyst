use cosmwasm_schema::cw_serde;
use cosmwasm_std::IbcEndpoint;
use cw_storage_plus::Map;

// Interface storage
pub const OPEN_CHANNELS: Map<&str, IbcChannelInfo> = Map::new("catalyst-ibc-interface-open-channels");

// Use a stripped down version of cosmwasm_std::IbcChannel to store the information of the
// interface's open channels.
#[cw_serde]
pub struct IbcChannelInfo {
    pub endpoint: IbcEndpoint,
    pub counterparty_endpoint: IbcEndpoint,
    pub connection_id: String
}

