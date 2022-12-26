use cosmwasm_std::StdError;
use thiserror::Error;

// TODO move to swap-pool-common?
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("GenericError")]    //TODO replace this error with a custom one
    GenericError {},

    #[error("ArithmeticError")]
    ArithmeticError {},

    #[error("Invalid assets (invalid number of assets or invalid asset address)")]
    InvalidAssets {},

    #[error("Invalid asset weights (invalid number of weights or zero valued weight provided)")]
    InvalidAssetWeights {},

    #[error("Amplification must be set to 1_x64 for non-amplified pools.")]
    InvalidAmplification {},

    #[error("Invalid IBC interface")]
    InvalidIBCInterface {},

    #[error("Invalid setup master")]
    InvalidSetupMaster {},

    #[error("Invalid assets balances: incorrect balances count or 0 balance provided.")]
    InvalidAssetsBalances {},


    // Swaps
    #[error("Swap yield is less than the specified minimum.")]
    SwapMinYieldNotFulfilled {},
    
    #[error("Swap amount exceeds pool limit.")]
    SwapLimitExceeded {},
    
    #[error("Liquidity swap amount exceeds pool limit.")]
    LiquiditySwapLimitExceeded {},
    


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
