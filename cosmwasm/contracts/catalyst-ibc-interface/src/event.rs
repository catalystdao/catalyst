use cosmwasm_std::{Event, Binary, Addr, Uint64};


/// Generate an event for a contract owner update.
/// 
/// # Arguments:
/// * `account` - The new owner.
/// 
pub fn set_owner_event(
    account: String
) -> Event {
    Event::new("set-owner")
        .add_attribute("account", account)
}


/// Generate an event for a swap underwrite.
/// 
/// # Arguments:
/// * `identifier` - The underwritten swap identifier.
/// * `underwriter` - The underwritter.
/// * `expiry` - Time at which the underwrite expires.
/// 
pub fn underwrite_swap_event(
    identifier: Binary,
    underwriter: Addr,
    expiry: Uint64
) -> Event {
    Event::new("underwrite-swap")
        .add_attribute("identifier", identifier.to_base64())
        .add_attribute("underwriter", underwriter)
        .add_attribute("expiry", expiry)
}


/// Generate an event for a fulfilled underwrite.
/// 
/// # Arguments:
/// * `identifier` - The fulfilled underwritten swap identifier.
/// 
pub fn fulfill_underwrite_event(
    identifier: Binary
) -> Event {
    Event::new("fulfill-underwrite")
        .add_attribute("identifier", identifier.to_base64())
}
