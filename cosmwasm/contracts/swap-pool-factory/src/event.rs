use cosmwasm_std::{Event, Uint64};
use swap_pool_common::event::format_vec_for_event;


pub fn deploy_vault_event(
    vault_code_id: u64,
    chain_interface: Option<String>,
    deployer: String,
    vault_address: String,
    assets: Vec<String>,
    k: u64
) -> Event {
    Event::new("deploy-vault")
        .add_attribute("vault_code_id", Uint64::new(vault_code_id))
        .add_attribute("chain_interface", chain_interface.unwrap_or("null".to_string()))
        .add_attribute("deployer", deployer)
        .add_attribute("vault_address", vault_address)
        .add_attribute("assets", format_vec_for_event(assets))
        .add_attribute("k", Uint64::new(k))
}

pub fn set_default_governance_fee_share_event(
    fee: Uint64
) -> Event {
    Event::new("set-default-governance-fee-share")
        .add_attribute("fee", fee)
}

pub fn set_owner_event(
    account: String
) -> Event {
    Event::new("set-owner")
        .add_attribute("account", account)
}