use cosmwasm_std::{StdError, Uint64};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Factory contract has no owner.")]
    NoOwner {},

    #[error("Invalid default governance fee")]
    InvalidDefaultGovernanceFeeShare { requested_fee: Uint64, max_fee: Uint64 },

    #[error("Submessage reply id unknown: {id}")]
    UnknownReplyId { id: u64 },

    #[error("Surplus of assets received by the vault.")]
    ReceivedAssetCountSurplus {},

    #[error("Shortage of assets received by the vault")]
    ReceivedAssetCountShortage {},

    #[error("Received asset is invalid: {reason}")]
    ReceivedAssetInvalid{ reason: String },
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}


impl From<vault_assets::error::AssetError> for ContractError {
    fn from(err: vault_assets::error::AssetError) -> Self {
        match err {
            vault_assets::error::AssetError::Std(error) => ContractError::Std(error),
            vault_assets::error::AssetError::ReceivedAssetCountSurplus {} => ContractError::ReceivedAssetCountSurplus {},
            vault_assets::error::AssetError::ReceivedAssetCountShortage {} => ContractError::ReceivedAssetCountShortage {},
            vault_assets::error::AssetError::ReceivedAssetInvalid { reason } => ContractError::ReceivedAssetInvalid { reason },
            other => ContractError::Std(other.into())   // This should never happen
        }
    }
}
