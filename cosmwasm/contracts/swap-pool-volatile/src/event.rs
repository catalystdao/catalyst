use cosmwasm_std::{Event, Uint64};
use swap_pool_common::event::format_vec_for_event;


pub fn set_weights_event(
    target_timestamp: u64,
    target_weights: Vec<u64>
) -> Event {
    Event::new("set-weights")
    .add_attribute("target_timestamp", Uint64::new(target_timestamp))
    .add_attribute("target_weights", format_vec_for_event(
        target_weights.iter().map(|weight| Uint64::new(*weight)).collect()      //TODO better approach? (make format_vec_for_event accept an iterator?)
    ))
}