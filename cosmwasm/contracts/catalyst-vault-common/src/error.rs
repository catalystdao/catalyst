use cosmwasm_std::{StdError, OverflowError, Uint64, Uint128, Binary, ConversionOverflowError, DivideByZeroError, Coin};
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

    #[error("Invalid assets (invalid number of assets or invalid asset.)")]
    InvalidAssets {},

    #[error("Invalid parameters: {reason}")]
    InvalidParameters { reason: String },

    #[error("The requested asset does not form part of the vault.")]
    AssetNotFound {},

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

    #[error("Expected gas not received: {gas}.")]
    GasNotReceived { gas: Coin },

    #[error("Not enough gas received: {received} (expected {expected}).")]
    NotEnoughGasReceived {
        received: Coin,
        expected: Coin,
    },

    #[error("Invalid amplification value.")]
    InvalidAmplification {},

    #[error("Invalid vault fee: requested fee is {requested_fee}, max allowed fee is {max_fee}.")]
    InvalidVaultFee { requested_fee: Uint64, max_fee: Uint64 },

    #[error("Invalid governance fee: requested fee is {requested_fee}, max allowed fee is {max_fee}.")]
    InvalidGovernanceFee { requested_fee: Uint64, max_fee: Uint64 },

    #[error("Invalid provided zero balance.")]
    InvalidZeroBalance {},

    #[error("Invalid weight.")]
    InvalidWeight {},

    #[error("Security limit exceeded by {overflow} amount.")]
    SecurityLimitExceeded { overflow: U256 },

    #[error("Return insufficient: output is {out}, minimum output is {min_out}.")]
    ReturnInsufficient { out: Uint128, min_out: Uint128 },

    #[error("Vault not connected (channel id: {channel_id}, vault: {vault}).")]
    VaultNotConnected { channel_id: String, vault: Binary },

    #[error("The vault only allows for local swaps, as it has no cross chain interface.")]
    VaultHasNoInterface {},

    #[error("A non zero withdraw ratio is specified after all units have been consumed.")]
    WithdrawRatioNotZero {},

    #[error("Not all withdrawal units have been consumed after all assets have been processed ({units} units left).")]
    UnusedUnitsAfterWithdrawal { units: U256 },

    #[error("Target time too short/long")]
    InvalidTargetTime {},



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
                                                                    // InvalidZeroAmount variant).
        }
    }
}


impl From<vault_assets::error::AssetError> for ContractError {
    fn from(err: vault_assets::error::AssetError) -> Self {
        match err {
            vault_assets::error::AssetError::Std(error) => ContractError::Std(error),
            vault_assets::error::AssetError::InvalidParameters { reason } => ContractError::InvalidParameters { reason },
            vault_assets::error::AssetError::AssetNotFound {} => ContractError::AssetNotFound {},
            vault_assets::error::AssetError::AssetNotReceived { asset } => ContractError::AssetNotReceived { asset },
            vault_assets::error::AssetError::AssetSurplusReceived {} => ContractError::AssetSurplusReceived {},
            vault_assets::error::AssetError::UnexpectedAssetAmountReceived {
                received_amount,
                expected_amount,
                asset
            } => ContractError::UnexpectedAssetAmountReceived {received_amount, expected_amount, asset},
            vault_assets::error::AssetError::GasNotReceived { gas } => ContractError::GasNotReceived { gas },
            vault_assets::error::AssetError::NotEnoughGasReceived { 
                received,
                expected
            } => ContractError::NotEnoughGasReceived { received, expected },
        }
    }
}

impl From<vault_token::error::VaultTokenError> for ContractError {
    fn from(err: vault_token::error::VaultTokenError) -> Self {
        match err {
            vault_token::error::VaultTokenError::Std(error) => ContractError::Std(error),
            other_err => ContractError::Error(other_err.to_string())
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

impl From<DivideByZeroError> for ContractError {
    fn from(_err: DivideByZeroError) -> Self {
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
