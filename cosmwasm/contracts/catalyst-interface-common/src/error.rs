use catalyst_types::Bytes32;
use cosmwasm_std::{StdError, Uint64, Binary, OverflowError, Uint128};
use thiserror::Error;
use vault_assets::error::AssetError;

/// Never is a placeholder to ensure no errors are returned.
#[derive(Error, Debug)]
pub enum Never {}

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Payload encoding failed.")]
    PayloadEncodingError {},

    #[error("Payload deoding failed.")]
    PayloadDecodingError {},

    #[error("Invalid Catalyst 65-byte encoded address.")]
    InvalidCatalystEncodedAddress {},

    #[error("Submessage reply id unknown: {id}")]
    UnknownReplyId { id: u64 },

    #[error("An underwrite for the given parameters does not exist, id: {id}")]
    UnderwriteDoesNotExist { id: Binary },

    #[error("The underwrite has not expired. Blocks remaining: {blocks_remaining}")]
    UnderwriteNotExpired { blocks_remaining: Uint64 },

    #[error("The specified max underwrite duration is too short (set {set_duration}, min {min_duration})")]
    MaxUnderwriteDurationTooShort {
        set_duration: Uint64,
        min_duration: Uint64
    },

    #[error("The specified max underwrite duration is too long (set {set_duration}, max {max_duration})")]
    MaxUnderwriteDurationTooLong {
        set_duration: Uint64,
        max_duration: Uint64
    },

    #[error("The swap has already been recently underwritten")]
    SwapRecentlyUnderwritten {},

    #[error("Vault not connected (channel id: {channel_id}, vault: {vault}).")]
    VaultNotConnected {
        channel_id: Bytes32,
        vault: Binary
    },

    #[error("The source cross chain interface is invalid.")]
    InvalidSourceInterface {},

    #[error("Insufficient incentive \"{description}\" (minimum: {minimum}, actual: {actual})")]
    NotEnoughIncentives {
        description: String,
        minimum: Uint128,
        actual: Uint128
    }
}


impl From<ContractError> for StdError {
    fn from(err: ContractError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}

impl From<AssetError> for ContractError {
    fn from(err: AssetError) -> Self {
        StdError::from(err).into()
    }
}

impl From<OverflowError> for ContractError {
    fn from(err: OverflowError) -> Self {
        StdError::from(err).into()
    }
}
