use cosmwasm_std::{Event, Uint64};

pub fn set_amplification_event(
    target_timestamp: Uint64,
    target_amplification: Uint64
) -> Event {
    Event::new("set-amplification")
        .add_attribute("target_timestamp", target_timestamp)
        .add_attribute("target_amplification", target_amplification)
}

