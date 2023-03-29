use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, IbcEndpoint};
use cw_storage_plus::{Item, Map};

pub const CATALYST_IBC_INTERFACE_STATE: Item<CatalystIBCInterfaceState> = Item::new("catalyst-ibc-interface-state");

pub const OPEN_CHANNELS: Map<&str, IbcChannelInfo> = Map::new("catalyst-ibc-interface-open-channels");

#[cw_serde]
pub struct CatalystIBCInterfaceState {
    pub admin: Addr,
    pub default_timeout: u64
}

#[cw_serde]
pub struct IbcChannelInfo {                 // This is a stripped down version of cosmwasm_std::IbcChannel
    pub endpoint: IbcEndpoint,
    pub counterparty_endpoint: IbcEndpoint,
    pub connection_id: String
}

