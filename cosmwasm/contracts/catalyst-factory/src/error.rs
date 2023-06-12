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
    UnknownReplyId { id: u64 }
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}
