use cosmwasm_std::{StdError, OverflowError, Uint64, Uint128, Binary, ConversionOverflowError};
use catalyst_types::U256;
use fixed_point_math::FixedPointMathError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Error: {0}")]
    Error (String),         // Error type for all the miscellaneous errors that do not have their own type.

    #[error("Arithmetic error")]
    ArithmeticError {},

    #[error("Invalid assets (invalid number of assets or invalid asset address)")]
    InvalidAssets {},

    #[error("Invalid parameters {reason}")]
    InvalidParameters { reason: String },

    #[error("The requested asset does not form part of the vault.")]
    AssetNotFound {},

    #[error("Amplification must be set to 1e18 for non-amplified vaults.")]
    InvalidAmplification {},

    #[error("Invalid vault fee")]
    InvalidVaultFee { requested_fee: Uint64, max_fee: Uint64 },

    #[error("Invalid governance fee")]
    InvalidGovernanceFee { requested_fee: Uint64, max_fee: Uint64 },

    #[error("Zero balance")]
    InvalidZeroBalance {},

    #[error("Weight")]
    InvalidWeight {},

    #[error("Security limit exceeded")]
    SecurityLimitExceeded { overflow: U256 },

    #[error("Return insufficient")]
    ReturnInsufficient { out: Uint128, min_out: Uint128 },

    #[error("Vault not connected")]
    VaultNotConnected { channel_id: String, vault: Binary },

    #[error("The vault only allows for local swaps, as it has no cross chain interface.")]
    VaultHasNoInterface {},

    #[error("A non zero withdraw ratio is specified after all units have been consumed.")]
    WithdrawRatioNotZero {},

    #[error("Not all withdrawal units have been consumed after all assets have been processed.")]
    UnusedUnitsAfterWithdrawal { units: U256 },

    #[error("Target time too short/long")]
    InvalidTargetTime,



    // CW20 Errors
    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[error("Invalid expiration")]
    InvalidExpiration {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Minting cannot exceed the cap")]
    CannotExceedCap {},

    #[error("Duplicate initial balance addresses")]
    DuplicateInitialBalanceAddresses {}
}


impl From<cw20_base::ContractError> for ContractError {
    fn from(err: cw20_base::ContractError) -> Self {
        match err {
            cw20_base::ContractError::Std(error) => ContractError::Std(error),
            cw20_base::ContractError::Unauthorized {} => ContractError::Unauthorized {},
            cw20_base::ContractError::CannotSetOwnAccount {} => ContractError::CannotSetOwnAccount {},
            cw20_base::ContractError::InvalidExpiration {} => ContractError::InvalidExpiration {},
            cw20_base::ContractError::NoAllowance {} => ContractError::NoAllowance {},
            cw20_base::ContractError::CannotExceedCap {} => ContractError::CannotExceedCap {},
            _ => ContractError::Error("cw20 error.".to_string())    // Match all other cw20_base errors for completeness. None of these
                                                                    // are expected to be encountered by the vaults (including the deprecated 
                                                                    // InvalidZeroAmount variant)
        }
    }
}

impl From<OverflowError> for ContractError {
    fn from(_err: OverflowError) -> Self {
        ContractError::ArithmeticError {}
    }
}

impl From<ConversionOverflowError> for ContractError {
    fn from(_err: ConversionOverflowError) -> Self {
        ContractError::ArithmeticError {}
    }
}

impl From<ContractError> for StdError {
    fn from(err: ContractError) -> StdError {
        StdError::GenericErr { msg: err.to_string() }
    }
}

impl From<FixedPointMathError> for ContractError {
    fn from(_err: FixedPointMathError) -> Self {
        ContractError::ArithmeticError {}
    }
}

