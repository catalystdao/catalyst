use crate::{U256, I256};

pub trait AsU256 {
    fn as_u256(self) -> U256;
}

pub trait AsI256 {
    fn as_i256(self) -> I256;
}