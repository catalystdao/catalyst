use cosmwasm_std::{StdError, OverflowError, Uint128};
use ethnum::U256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("GenericError")]    //TODO replace this error with a custom one
    GenericError {},

    #[error("Arithmetic error")]
    ArithmeticError {},

    #[error("Invalid assets (invalid number of assets or invalid asset address)")]
    InvalidAssets {},

    #[error("Amplification must be set to 1_x64 for non-amplified pools.")]
    InvalidAmplification {},

    #[error("Invalid pool fee")]
    InvalidPoolFee { requested_fee: u64, max_fee: u64 },

    #[error("Invalid governance fee")]
    InvalidGovernanceFee { requested_fee: u64, max_fee: u64 },

    #[error("Security limit exceeded")]
    SecurityLimitExceeded { units: U256, capacity: U256 },


    #[error("Return insufficient")]
    ReturnInsufficient { out: Uint128, min_out: Uint128 },

    #[error("Pool not connected")]
    PoolNotConnected { channel_id: String, pool: String },

    #[error("The pool only allows for local swaps, as it has no cross chain interface.")]
    PoolHasNoInterface {},

    #[error("A non zero withdraw ratio is specified after all units have been consumed.")]
    WithdrawRatioNotZero { ratio: u64 },    //TODO EVM mismatch

    #[error("Not all withdrawal units have been consumed after all assets have been processed.")]
    UnusedUnitsAfterWithdrawal { units: U256 },




    // CW20 Errors
    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("Invalid expiration")]
    InvalidExpiration {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Minting cannot exceed the cap")]
    CannotExceedCap {},

    #[error("Duplicate initial balance addresses")]
    DuplicateInitialBalanceAddresses {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}


impl From<cw20_base::ContractError> for ContractError {
    fn from(err: cw20_base::ContractError) -> Self {
        match err {
            cw20_base::ContractError::Std(error) => ContractError::Std(error),
            cw20_base::ContractError::Unauthorized {} => ContractError::Unauthorized {},
            cw20_base::ContractError::CannotSetOwnAccount {} => ContractError::CannotSetOwnAccount {},
            cw20_base::ContractError::InvalidExpiration {} => ContractError::InvalidExpiration {},
            cw20_base::ContractError::InvalidZeroAmount {} => ContractError::InvalidZeroAmount {},
            cw20_base::ContractError::Expired {} => ContractError::Expired {},
            cw20_base::ContractError::NoAllowance {} => ContractError::NoAllowance {},
            cw20_base::ContractError::CannotExceedCap {} => ContractError::CannotExceedCap {},
            // This should never happen, as this contract doesn't use logo
            cw20_base::ContractError::LogoTooBig {}
            | cw20_base::ContractError::InvalidPngHeader {}
            | cw20_base::ContractError::InvalidXmlPreamble {} => {
                ContractError::Std(StdError::generic_err(err.to_string()))
            }
            cw20_base::ContractError::DuplicateInitialBalanceAddresses {} => {
                ContractError::DuplicateInitialBalanceAddresses {}
            }
        }
    }
}

impl From<OverflowError> for ContractError {
    fn from(_err: OverflowError) -> Self {
        ContractError::ArithmeticError {}
    }
}

//TODO replace these (i.e. ()) errors with other ones?
impl From<()> for ContractError {
    fn from(_err: ()) -> Self {
        ContractError::GenericError {}
    }
}

//TODO overhaul
impl From<ContractError> for StdError {
    fn from(_err: ContractError) -> StdError {
        StdError::GenericErr { msg: "".to_owned() } //TODO error (use _err)
    }
}