use cosmwasm_std::StdError;
use thiserror::Error;

/// Never is a placeholder to ensure we don't return any errors
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

    #[error("Submessage reply id unknown: {id}")]
    UnknownReplyId { id: u64 },

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
