use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssetError {

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid parameters: {reason}")]
    InvalidParameters { reason: String },

    #[error("The requested asset does not form part of the vault.")]
    AssetNotFound {},

    #[error("Expected asset not received: {asset}.")]
    AssetNotReceived { asset: String },

    #[error("Asset surplus received.")]
    AssetSurplusReceived {},

    #[error("Invalid amount {received_amount} for asset {asset} received (expected {expected_amount}).")]
    UnexpectedAssetAmountReceived {
        received_amount: Uint128,
        expected_amount: Uint128,
        asset: String
    },

}

impl From<AssetError> for StdError {
    fn from(err: AssetError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}
