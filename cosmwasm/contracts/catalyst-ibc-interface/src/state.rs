use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const CATALYST_IBC_INTERFACE_STATE: Item<CatalystIBCInterfaceState> = Item::new("catalyst-ibc-interface-state");

#[cw_serde]
pub struct CatalystIBCInterfaceState {
    pub ibc_endpoint: Addr
}