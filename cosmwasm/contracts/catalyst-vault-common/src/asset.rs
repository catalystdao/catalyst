
#[cfg(all(not(feature="asset_native"), not(feature="asset_cw20")))]
compile_error!("An asset-type feature must be enabled (\"asset_native\" or \"asset_cw20\")");

#[cfg(all(feature="asset_native", feature="asset_cw20"))]
compile_error!("Multiple asset-type features cannot be enabled at the same time (\"asset_native\" and \"asset_cw20\")");

#[cfg(feature="asset_native")]
pub use vault_assets::asset::asset_native::NativeAsset as Asset;
#[cfg(feature="asset_native")]
pub use vault_assets::asset::asset_native::NativeVaultAssets as VaultAssets;

#[cfg(feature="asset_cw20")]
pub use vault_assets::asset::asset_cw20::Cw20Asset as Asset;
#[cfg(feature="asset_cw20")]
pub use vault_assets::asset::asset_cw20::Cw20VaultAssets as VaultAssets;

pub use vault_assets::asset::{VaultAssetsTrait, AssetTrait};