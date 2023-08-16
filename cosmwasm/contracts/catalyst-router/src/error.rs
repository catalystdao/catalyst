use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unrecognized command id: {command_id}")]
    InvalidCommand{
        command_id: u8
    },

    #[error("Command {index} reverted: {error}")]
    CommandReverted{
        index: u64,
        error: String
    },

    #[error("The router does not implement any queries.")]
    NoQueries {}
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}
