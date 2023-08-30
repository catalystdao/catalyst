use cosmwasm_std::{StdError, Uint64, Uint128};
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

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}


impl From<vault_assets::error::AssetError> for ContractError {
    fn from(err: vault_assets::error::AssetError) -> Self {
        match err {
            vault_assets::error::AssetError::Std(error) => ContractError::Std(error),
            vault_assets::error::AssetError::AssetNotReceived { asset } => ContractError::AssetNotReceived { asset },
            vault_assets::error::AssetError::AssetSurplusReceived {} => ContractError::AssetSurplusReceived {},
            vault_assets::error::AssetError::UnexpectedAssetAmountReceived {
                received_amount,
                expected_amount,
                asset
            } => ContractError::UnexpectedAssetAmountReceived {received_amount, expected_amount, asset},
            other => ContractError::Std(other.into())   // This should never happen
        }
    }
}
