#[cfg(feature="amplification_update")]
use cosmwasm_std::{Event, Uint64};

/// Generate the event of the amplification update.
/// 
/// # Arguments:
/// * `target_timestamp` - The time at which the amplification update must be completed.
/// * `target_amplification` - The new target amplification.
/// 
#[cfg(feature="amplification_update")]
pub fn set_amplification_event(
    target_timestamp: Uint64,
    target_amplification: Uint64
) -> Event {
    Event::new("set-amplification")
        .add_attribute("target_timestamp", target_timestamp)
        .add_attribute("target_amplification", target_amplification)
}

