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

    #[error("Invalid pool fee")]
    InvalidPoolFee { requested_fee: u64, max_fee: u64 },

    #[error("Invalid governance fee")]
    InvalidGovernanceFee { requested_fee: u64, max_fee: u64 },


    // Swaps
    #[error("Swap yield is less than the specified minimum.")]
    SwapMinYieldNotFulfilled {},
    
    #[error("Swap amount exceeds pool limit.")]
    SwapLimitExceeded {},
    
    #[error("Liquidity swap amount exceeds pool limit.")]
    LiquiditySwapLimitExceeded {},
    
    #[error("Resulting escrow hash already in use.")]
    NonEmptyEscrow {},
    


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

//TODO overhaul error definitions (currently there is a mixture of definitions across contracts)
impl From<swap_pool_common::ContractError> for ContractError {
    fn from(err: swap_pool_common::ContractError) -> Self {
        match err {
            swap_pool_common::ContractError::Std(error) => ContractError::Std(error),
            swap_pool_common::ContractError::Unauthorized {} => ContractError::Unauthorized {},
            swap_pool_common::ContractError::ArithmeticError {} => ContractError::ArithmeticError {},
            swap_pool_common::ContractError::InvalidAssets {} => ContractError::InvalidAssets {},
            swap_pool_common::ContractError::InvalidPoolFee { requested_fee, max_fee }
                => ContractError::InvalidPoolFee {requested_fee, max_fee},
            swap_pool_common::ContractError::InvalidGovernanceFee { requested_fee, max_fee }
                => ContractError::InvalidGovernanceFee {requested_fee, max_fee},
            swap_pool_common::ContractError::CannotSetOwnAccount {} => ContractError::CannotSetOwnAccount {},
            swap_pool_common::ContractError::InvalidExpiration {} => ContractError::InvalidExpiration {},
            swap_pool_common::ContractError::InvalidZeroAmount {} => ContractError::InvalidZeroAmount {},
            swap_pool_common::ContractError::Expired {} => ContractError::Expired {},
            swap_pool_common::ContractError::NoAllowance {} => ContractError::NoAllowance {},
            swap_pool_common::ContractError::CannotExceedCap {} => ContractError::CannotExceedCap {},
            swap_pool_common::ContractError::DuplicateInitialBalanceAddresses {} => ContractError::DuplicateInitialBalanceAddresses {}
        }
    }
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
