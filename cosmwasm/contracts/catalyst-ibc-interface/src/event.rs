use cosmwasm_std::Event;


/// Generate an event for a contract owner update.
/// 
/// # Arguments
/// 
/// * `account` - The new owner.
/// 
pub fn set_owner_event(
    account: String
) -> Event {
    Event::new("set-owner")
        .add_attribute("account", account)
}