use cosmwasm_std::{Event, Uint64, Uint128};
use catalyst_vault_common::event::format_vec_for_event;


pub fn set_weights_event(
    target_timestamp: Uint64,
    target_weights: Vec<Uint128>
) -> Event {
    Event::new("set-weights")
    .add_attribute("target_timestamp", target_timestamp)
    .add_attribute("target_weights", format_vec_for_event(
        target_weights.iter().map(|weight| *weight).collect()      //TODO better approach? (make format_vec_for_event accept an iterator?)
    ))
}