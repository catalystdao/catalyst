use cosmwasm_std::{Event, Uint64};
use catalyst_vault_common::{event::format_vec_for_event, asset::Asset};


/// Generate an event for vault deployment.
/// 
/// # Arguments
/// 
/// * `vault_code_id` - The code id of the *stored* contract with which to deploy the new vault.
/// * `chain_interface` - The interface used for cross-chain swaps. It can be set to None to disable cross-chain swaps.
/// * `deployer` - The deployer of the vault.
/// * `vault_address` - The address of the deployed vault.
/// * `assets` - A list of the assets that are to be supported by the vault.
/// * `k` - A vault configuration parameter (currently set to the amplification value).
/// 
pub fn deploy_vault_event(
    vault_code_id: u64,
    chain_interface: Option<String>,
    deployer: String,
    vault_address: String,
    assets: Vec<Asset>,
    k: Uint64
) -> Event {
    Event::new("deploy-vault")
        .add_attribute("vault_code_id", Uint64::new(vault_code_id))
        .add_attribute("chain_interface", chain_interface.unwrap_or("null".to_string()))
        .add_attribute("deployer", deployer)
        .add_attribute("vault_address", vault_address)
        .add_attribute("assets", format_vec_for_event(assets))
        .add_attribute("k", k)      // NOTE: named 'k' to match the EVM implementation.
}

/// Generate an event for a governance fee share update.
/// 
/// # Arguments
/// 
/// * `fee` - The new governance fee share (18 decimals).
/// 
pub fn set_default_governance_fee_share_event(
    fee: Uint64
) -> Event {
    Event::new("set-default-governance-fee-share")
        .add_attribute("fee", fee)
}

/// Generate an event for a contract owner update.
/// 
/// # Arguments
/// 
/// * `account` - The new factory owner.
/// 
pub fn set_owner_event(
    account: String
) -> Event {
    Event::new("set-owner")
        .add_attribute("account", account)
}