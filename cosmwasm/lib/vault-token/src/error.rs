use cosmwasm_std::{StdError, OverflowError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultTokenError {

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Mint failed: {reason}")]
    MintFailed { reason: String },

    #[error("Burn failed: {reason}")]
    BurnFailed {reason: String}
}

impl From<VaultTokenError> for StdError {
    fn from(err: VaultTokenError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}

impl From<OverflowError> for VaultTokenError {
    fn from(value: OverflowError) -> Self {
        VaultTokenError::Std(value.into())
    }
}