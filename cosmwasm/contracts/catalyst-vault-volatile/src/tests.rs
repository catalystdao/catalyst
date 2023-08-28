mod helpers;
mod parameters;
// mod test_deposit;
mod test_fees;
mod test_finish_setup;
mod test_initialize_swap_curves;
mod test_instantiate;
mod test_local_swap;
mod test_receive_asset;
// mod test_receive_liquidity;
mod test_security_limit;
mod test_send_asset;
// mod test_send_asset_success_failure;
mod test_send_liquidity;
// mod test_send_liquidity_success_failure;
mod test_vault_connections;
mod test_weights_update;
mod test_withdraw_even;
mod test_withdraw_mixed;

#[cfg(feature="asset_native")]
pub use test_helpers::asset::TestNativeAsset as TestAsset;
#[cfg(feature="asset_native")]
pub use test_helpers::env::env_native_asset::TestNativeAssetEnv as TestEnv;

#[cfg(feature="asset_cw20")]
pub use test_helpers::asset::TestCw20Asset as TestAsset;
#[cfg(feature="asset_cw20")]
pub use test_helpers::env::env_cw20_asset::TestCw20AssetEnv as TestEnv;
