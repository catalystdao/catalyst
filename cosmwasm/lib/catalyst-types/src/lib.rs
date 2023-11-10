mod bytes32;
mod unsigned256;
mod signed256;
mod traits;

#[doc(hidden)]
pub mod macros;

pub use crate::{
    bytes32::Bytes32,
    unsigned256::U256,
    signed256::I256,
    traits::AsU256,
    traits::AsI256
};

/// Re-export cosmwasm_std errors
pub mod errors {
    pub use cosmwasm_std::{OverflowError, DivideByZeroError, ConversionOverflowError};
    use thiserror::Error;

    // NOTE: This error should be imported directly from 'cosmwasm_std', but as of version 1.3.0
    // the error is not exposed.
    #[derive(Error, Debug, PartialEq, Eq)]
    pub enum DivisionError {
        #[error("Divide by zero")]
        DivideByZero,

        #[error("Overflow in division")]
        Overflow,
    }
}