use cosmwasm_std::{Uint128, Event, Binary, Uint64};
use catalyst_types::U256;

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
        .add_attribute("to_asset_index", to_asset_index.to_string())    //TODO format
        .add_attribute("from_amount", from_amount)
        .add_attribute("min_out", min_out)
        .add_attribute("units", units)
        .add_attribute("fee", fee)
}

pub fn receive_asset_event(
    channel_id: String,
    from_vault: Binary,
    to_account: String,
    to_asset: String,
    units: U256,
    to_amount: Uint128,
    from_amount: U256,
    from_asset: Binary,
    source_block_number_mod: u32
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
        .add_attribute("source_block_number_mod", source_block_number_mod.to_string())  //TODO format   //TODO should be 'from' and not 'source'
}

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

pub fn receive_liquidity_event(
    channel_id: String,
    from_vault: Binary,
    to_account: String,
    units: U256,
    to_amount: Uint128,
    from_amount: U256,
    source_block_number_mod: u32
) -> Event {
    Event::new("receive-liquidity")
        .add_attribute("channel_id", channel_id)
        .add_attribute("from_vault", from_vault.to_base64())
        .add_attribute("to_account", to_account)
        .add_attribute("units", units)
        .add_attribute("to_amount", to_amount)
        .add_attribute("from_amount", from_amount)
        .add_attribute("source_block_number_mod", source_block_number_mod.to_string())  //TODO format   //TODO should be 'from' and not 'source'
}

pub fn deposit_event(
    to_account: String,
    mint: Uint128,
    deposit_amounts: Vec<Uint128>
) -> Event {
    Event::new("deposit")
        .add_attribute("to_account", to_account)
        .add_attribute("mint", mint)
        .add_attribute("assets", format_vec_for_event(deposit_amounts))
}

pub fn withdraw_event(
    to_account: String,
    burn: Uint128,
    withdraw_amounts: Vec<Uint128>
) -> Event {
    Event::new("deposit")
        .add_attribute("to_account", to_account)
        .add_attribute("burn", burn)
        .add_attribute("assets", format_vec_for_event(withdraw_amounts))
}

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
        .add_attribute("block_number_mod", block_number_mod.to_string())    //TODO format
}

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
        .add_attribute("block_number_mod", block_number_mod.to_string())    //TODO format
}

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
        .add_attribute("block_number_mod", block_number_mod.to_string())    //TODO format
}

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
        .add_attribute("block_number_mod", block_number_mod.to_string())    //TODO format
}

pub fn finish_setup_event() -> Event {
    Event::new("finish-setup")
}

pub fn set_fee_administrator_event(
    administrator: String
) -> Event {
    Event::new("set-fee-administrator")
        .add_attribute("administrator", administrator)
}

pub fn set_vault_fee_event(
    fee: Uint64
) -> Event {
    Event::new("set-vault-fee")
        .add_attribute("fee", fee)
}

pub fn set_governance_fee_share_event(
    fee: Uint64
) -> Event {
    Event::new("set-governance-fee-share")
        .add_attribute("fee", fee)
}

pub fn set_connection_event(
    channel_id: String,
    to_vault: Binary,
    state: bool
) -> Event {
    Event::new("set-connection")
        .add_attribute("channel_id", channel_id)
        .add_attribute("to_vault", to_vault.to_base64())
        .add_attribute("state", state.to_string())
}


// Misc helpers *****************************************************************************************************************
//TODO move helper somewhere else? (To reuse across implementations)
pub fn format_vec_for_event<T: ToString>(vec: Vec<T>) -> String {
    //TODO review output format
    vec
        .iter()
        .map(T::to_string)
        .collect::<Vec<String>>().join(", ")
}