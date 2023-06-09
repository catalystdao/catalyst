use cosmwasm_std::{StdError, Uint64};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid default governance fee")]
    InvalidDefaultGovernanceFeeShare { requested_fee: Uint64, max_fee: Uint64 },

    #[error("Submessage reply id unknown: {id}")]
    UnknownReplyId { id: u64 }

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
