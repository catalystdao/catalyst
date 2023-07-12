use cosmwasm_std::{Event, Uint64, Uint128};
use catalyst_vault_common::event::format_vec_for_event;


/// Generate the event of a weights update.
/// 
/// # Arguments:
/// * `target_timestamp` - The time at which the weights update must be completed.
/// * `target_weights` - The new target weights.
/// 
pub fn set_weights_event(
    target_timestamp: Uint64,
    target_weights: Vec<Uint128>
) -> Event {
    Event::new("set-weights")
    .add_attribute("target_timestamp", target_timestamp)
    .add_attribute("target_weights", format_vec_for_event(target_weights))
}