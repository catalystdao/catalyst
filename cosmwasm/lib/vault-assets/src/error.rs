use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssetError {

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid parameters: {reason}")]
    InvalidParameters { reason: String },

    #[error("The requested asset does not form part of the vault.")]
    AssetNotFound {},

    #[error("Surplus of assets received by the vault.")]
    ReceivedAssetCountSurplus {},

    #[error("Shortage of assets received by the vault")]
    ReceivedAssetCountShortage {},

    #[error("Received asset is invalid: {reason}")]
    ReceivedAssetInvalid{ reason: String },

}

impl From<AssetError> for StdError {
    fn from(err: AssetError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}
