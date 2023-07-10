use cosmwasm_std::{Uint128, Event, Binary, Uint64, Response};
use catalyst_types::U256;


/// Generate the event of a local swap.
/// 
/// # Arguments:
/// * `account` - The account which has executed the swap.
/// * `from_asset` - The source asset.
/// * `to_asset` - The destination asset.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `to_amount` - The `to_asset` amount bought from the vault.
/// 
pub fn local_swap_event(
    account: String,
    from_asset: String,
    to_asset: String,
    from_amount: Uint128,
    to_amount: Uint128
) -> Event {
    Event::new("local-swap")
        .add_attribute("account", account)
        .add_attribute("from_asset", from_asset)
        .add_attribute("to_asset", to_asset)
        .add_attribute("from_amount", from_amount)
        .add_attribute("to_amount", to_amount)
}


/// Generate the event of the initiation of an asset cross-chain swap.
/// 
/// # Arguments:
/// * `channel_id` - The target chain identifier.
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `from_asset` - The source asset.
/// * `to_asset_index` - The destination asset index.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `min_out` - The mininum `to_asset` output amount to get on the target vault.
/// * `units` - The amount of units bought.
/// * `fee` - The amount of `from_asset` paid to the vault in fees.
/// 
pub fn send_asset_event(
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    from_asset: String,
    to_asset_index: u8,
    from_amount: Uint128,
    min_out: U256,
    units: U256,
    fee: Uint128
) -> Event {
    Event::new("send-asset")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_vault", to_vault.to_base64())
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("from_asset", from_asset)
        .add_attribute("to_asset_index", to_asset_index.to_string())
        .add_attribute("from_amount", from_amount)
        .add_attribute("min_out", min_out)
        .add_attribute("units", units)
        .add_attribute("fee", fee)
}


/// Generate the event of the completion of an asset cross-chain swap.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `from_vault` - The source vault on the source chain.
/// * `to_account` - The recipient of the swap.
/// * `to_asset` - The destination asset.
/// * `units` - The incoming units.
/// * `to_amount` - The `to_asset` amount bought from the vault.
/// * `from_amount` - The `from_asset` amount sold to the source vault.
/// * `from_asset` - The source asset of the source vault.
/// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn receive_asset_event(
    channel_id: String,
    from_vault: Binary,
    to_account: String,
    to_asset: String,
    units: U256,
    to_amount: Uint128,
    from_amount: U256,
    from_asset: Binary,
    from_block_number_mod: u32
) -> Event {
    Event::new("receive-asset")
        .add_attribute("channel_id", channel_id)
        .add_attribute("from_vault", from_vault.to_base64())
        .add_attribute("to_account", to_account)
        .add_attribute("to_asset", to_asset)
        .add_attribute("units", units)
        .add_attribute("to_amount", to_amount)
        .add_attribute("from_amount", from_amount)
        .add_attribute("from_asset", from_asset.to_base64())
        .add_attribute("from_block_number_mod", from_block_number_mod.to_string())
}


/// Generate the event of the initiation of a liquidity cross-chain swap.
/// 
/// # Arguments:
/// * `channel_id` - The target chain identifier.
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `from_amount` - The vault tokens amount sold to the vault.
/// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
/// * `min_reference_asset` - The mininum reference asset value on the target vault.
/// * `units` - The amount of units bought.
/// 
pub fn send_liquidity_event(
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    from_amount: Uint128,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    units: U256
) -> Event {
    Event::new("send-liquidity")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_vault", to_vault.to_base64())
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("from_amount", from_amount)
        .add_attribute("min_vault_tokens", min_vault_tokens)
        .add_attribute("min_reference_asset", min_reference_asset)
        .add_attribute("units", units)
}


/// Generate the event of the completion of a liquidity cross-chain swap.
/// 
/// # Arguments:
/// * `channel_id` - The source chain identifier.
/// * `from_vault` - The source vault on the source chain.
/// * `to_account` - The recipient of the swap.
/// * `units` - The incoming units.
/// * `to_amount` - The vault token amount bought from the vault.
/// * `from_amount` - The vault token amount sold to the source vault.
/// * `from_block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn receive_liquidity_event(
    channel_id: String,
    from_vault: Binary,
    to_account: String,
    units: U256,
    to_amount: Uint128,
    from_amount: U256,
    from_block_number_mod: u32
) -> Event {
    Event::new("receive-liquidity")
        .add_attribute("channel_id", channel_id)
        .add_attribute("from_vault", from_vault.to_base64())
        .add_attribute("to_account", to_account)
        .add_attribute("units", units)
        .add_attribute("to_amount", to_amount)
        .add_attribute("from_amount", from_amount)
        .add_attribute("from_block_number_mod", from_block_number_mod.to_string())
}


/// Generate the event of a deposit.
/// 
/// # Arguments:
/// * `to_account` - The depositor account.
/// * `mint` - The amount of vault tokens minted to the depositor.
/// * `deposit_amounts` - A list of the deposited amounts.
/// 
pub fn deposit_event(
    to_account: String,
    mint: Uint128,
    deposit_amounts: Vec<Uint128>
) -> Event {
    Event::new("deposit")
        .add_attribute("to_account", to_account)
        .add_attribute("mint", mint)
        .add_attribute("deposit_amounts", format_vec_for_event(deposit_amounts))
}


/// Generate the event of a withdrawal.
/// 
/// # Arguments:
/// * `to_account` - The withdrawer account.
/// * `burn` - The amount of vault tokens burnt from the depositor.
/// * `withdraw_amounts` - A list of the withdrawn amounts.
/// 
pub fn withdraw_event(
    to_account: String,
    burn: Uint128,
    withdraw_amounts: Vec<Uint128>
) -> Event {
    Event::new("deposit")
        .add_attribute("to_account", to_account)
        .add_attribute("burn", burn)
        .add_attribute("withdraw_amounts", format_vec_for_event(withdraw_amounts))
}


/// Generate the event of a 'successful' asset swap confirmation.
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `units` - The units value of the swap.
/// * `escrow_amount` - The escrowed asset amount.
/// * `asset` - The swap source asset.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn send_asset_success_event(
    channel_id: String,
    to_account: Binary,
    units: U256,
    escrow_amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Event {
    Event::new("send-asset-success")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("units", units)
        .add_attribute("escrow_amount", escrow_amount)
        .add_attribute("asset", asset)
        .add_attribute("block_number_mod", block_number_mod.to_string())
}


/// Generate the event of an 'unsuccessful' asset swap confirmation.
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `units` - The units value of the swap.
/// * `escrow_amount` - The escrowed asset amount.
/// * `asset` - The swap source asset.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn send_asset_failure_event(
    channel_id: String,
    to_account: Binary,
    units: U256,
    escrow_amount: Uint128,
    asset: String,
    block_number_mod: u32
) -> Event {
    Event::new("send-asset-failure")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("units", units)
        .add_attribute("escrow_amount", escrow_amount)
        .add_attribute("asset", asset)
        .add_attribute("block_number_mod", block_number_mod.to_string())
}


/// Generate the event of a 'successful' liquidity swap confirmation.
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `units` - The units value of the swap.
/// * `escrow_amount` - The escrowed vault tokens amount.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn send_liquidity_success_event(
    channel_id: String,
    to_account: Binary,
    units: U256,
    escrow_amount: Uint128,
    block_number_mod: u32
) -> Event {
    Event::new("send-liquidity-success")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("units", units)
        .add_attribute("escrow_amount", escrow_amount)
        .add_attribute("block_number_mod", block_number_mod.to_string())
}


/// Generate the event of an 'unsuccessful' liquidity swap confirmation.
/// 
/// # Arguments:
/// * `channel_id` - The swap's channel id.
/// * `to_account` - The recipient of the swap output.
/// * `units` - The units value of the swap.
/// * `escrow_amount` - The escrowed vault tokens amount.
/// * `block_number_mod` - The block number at which the swap transaction was commited (modulo 2^32).
/// 
pub fn send_liquidity_failure_event(
    channel_id: String,
    to_account: Binary,
    units: U256,
    escrow_amount: Uint128,
    block_number_mod: u32
) -> Event {
    Event::new("send-liquidity-failure")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_account", to_account.to_base64())
        .add_attribute("units", units)
        .add_attribute("escrow_amount", escrow_amount)
        .add_attribute("block_number_mod", block_number_mod.to_string())
}


/// Generate the event for when vault setup is complete.
pub fn finish_setup_event() -> Event {
    Event::new("finish-setup")
}


/// Generate the event for when the fee administrator is set.
/// 
/// # Arguments:
/// * `administrator` - The new fee administrator.
/// 
pub fn set_fee_administrator_event(
    administrator: String
) -> Event {
    Event::new("set-fee-administrator")
        .add_attribute("administrator", administrator)
}


/// Generate the event for when the vault fee is set.
/// 
/// # Arguments:
/// * `fee` - The new vault fee.
/// 
pub fn set_vault_fee_event(
    fee: Uint64
) -> Event {
    Event::new("set-vault-fee")
        .add_attribute("fee", fee)
}


/// Generate the event for when the governance fee share is set.
/// 
/// # Arguments:
/// * `fee` - The new governance fee share.
/// 
pub fn set_governance_fee_share_event(
    fee: Uint64
) -> Event {
    Event::new("set-governance-fee-share")
        .add_attribute("fee", fee)
}


/// Generate the event for when a vault connection is set.
/// 
/// # Arguments:
/// * `channel_id` - The channel id that connects with the remoute vault.
/// * `vault` - The remote vault address to be connected to this vault.
/// * `state` - Whether the connection is enabled.
/// 
pub fn set_connection_event(
    channel_id: String,
    vault: Binary,
    state: bool
) -> Event {
    Event::new("set-connection")
        .add_attribute("channel_id", channel_id)
        .add_attribute("vault", vault.to_base64())
        .add_attribute("state", state.to_string())
}


// Misc helpers *****************************************************************************************************************

/// Format a vector into a comma separated list.
pub fn format_vec_for_event<T: ToString>(vec: Vec<T>) -> String {
    vec
        .iter()
        .map(T::to_string)
        .collect::<Vec<String>>().join(", ")
}

/// Transform a cw20 'Response' into an event. This is to be used for including the responses 
/// returned by the burn/mint cw20 functions on the vault responses (on deposits and withdrawals).
pub fn cw20_response_to_standard_event(response: Response) -> Event {
    Event::new("vault-token")
        .add_attributes(response.attributes)
}