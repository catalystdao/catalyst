use catalyst_interface_common::catalyst_payload::CatalystEncodedAddress;
use catalyst_types::Bytes32;
use cosmwasm_std::{Uint64, Event, Binary, Uint128};


pub fn set_min_gas_for_event(
    channel_id: Bytes32,
    min_gas: Uint64
) -> Event {
    Event::new("min-gas-for")
        .add_attribute("chain_identifier", channel_id.to_base64())
        .add_attribute("min_gas", min_gas)
}

pub fn set_min_ack_gas_price_event(
    min_ack_gas_price: Uint128
) -> Event {
    Event::new("min-ack-gas-price")
        .add_attribute("min_ack_gas_price", min_ack_gas_price)
}

pub fn remote_implementation_set_event(
    channel_id: Bytes32,
    remote_interface: CatalystEncodedAddress,
    remote_gi: Binary
) -> Event {
    Event::new("remote-implementation-set")
        .add_attribute("channel_id", channel_id.to_base64())
        .add_attribute("remote_interface", remote_interface.to_binary().to_base64())
        .add_attribute("remote_gi", remote_gi.to_base64())
}