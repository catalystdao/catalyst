use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid default governance fee")]
    InvalidDefaultGovernanceFeeShare { requested_fee: u64, max_fee: u64 },

    #[error("Submessage reply id unknown: {id}")]
    UnknownReplyId { id: u64 }

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
