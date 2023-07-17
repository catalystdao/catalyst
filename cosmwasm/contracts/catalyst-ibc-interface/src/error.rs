use cosmwasm_std::StdError;
use thiserror::Error;

/// Never is a placeholder to ensure no errors are returned.
#[derive(Error, Debug)]
pub enum Never {}

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Only IBC channel version 'catalyst-v1' is supported, got {version}.")]
    InvalidIbcChannelVersion { version: String },

    #[error("Payload encoding failed.")]
    PayloadEncodingError {},

    #[error("Payload deoding failed.")]
    PayloadDecodingError {},

    #[error("Invalid Catalyst 65-byte encoded address.")]
    InvalidCatalystEncodedAddress {},

    #[error("Submessage reply id unknown: {id}")]
    UnknownReplyId { id: u64 }
}
