use forward_ref::{forward_ref_binop, forward_ref_op_assign};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, Shr, ShrAssign, Sub,
    SubAssign, BitOr, BitOrAssign, BitAndAssign, BitAnd,
};
use std::str::FromStr;

use cosmwasm_std::{forward_ref_partial_eq, StdError, Uint128, OverflowError, OverflowOperation, DivideByZeroError, Uint64, ConversionOverflowError};

/// Used internally - we don't want to leak this type since we might change
/// the implementation in the future.
use ethnum::U256 as BaseU256;

use crate::I256;
use crate::traits::{AsI256, AsU256};

/// An implementation of u256 that is using strings for JSON encoding/decoding,
/// such that the full u256 range can be used for clients that convert JSON numbers to floats,
/// like JavaScript and jq.
///
/// This implementation is based on CosmWasm's Uint implementations
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct U256(#[schemars(with = "String")] BaseU256);

forward_ref_partial_eq!(U256, U256);

impl U256 {
    pub const MAX: U256 = U256(BaseU256::MAX);
    pub const MIN: U256 = U256(BaseU256::MIN);

    // Note: this method is provided for maximum efficiency when forming the base structure (ethnum::U256).
    pub const fn from_words(hi: u128, lo: u128) -> Self {
        Self(BaseU256::from_words(hi, lo))
    }

    // Note: this method is provided for maximum efficiency when decomposing the base structure (ethnum::U256).
    pub const fn into_words(self) -> (u128, u128) {
        self.0.into_words()
    }

    /// Creates a U256(0)
    #[inline]
    pub const fn zero() -> Self {
        Self(BaseU256::ZERO)
    }

    /// Creates a U256(1)
    #[inline]
    pub const fn one() -> Self {
        Self(BaseU256::ONE)
    }

    #[must_use]
    pub fn from_be_bytes(data: [u8; 32]) -> Self {
        Self(BaseU256::from_be_bytes(data))
    }

    #[must_use]
    pub fn from_le_bytes(data: [u8; 32]) -> Self {
        Self(BaseU256::from_le_bytes(data))
    }

    /// A conversion from `u128` that, unlike the one provided by the `From` trait,
    /// can be used in a `const` context.
    #[must_use]
    pub const fn from_u128(num: u128) -> Self {
        Self(BaseU256::from_words(0, num))
    }

    /// A conversion from `Uint128` that, unlike the one provided by the `From` trait,
    /// can be used in a `const` context.
    #[must_use]
    pub const fn from_uint128(num: Uint128) -> Self {
        Self::from_u128(num.u128())
    }

    #[must_use]
    pub const fn as_uint128(self) -> Uint128 {
        Uint128::new(self.0.as_u128())
    }

    #[must_use]
    pub const fn as_u8(self) -> u8 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_u16(self) -> u16 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_u32(self) -> u32 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_u128(self) -> u128 {
        let (_, lo) = self.0.into_words();
        lo
    }


    #[must_use]
    pub const fn as_i8(self) -> i8 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_i16(self) -> i16 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_i32(self) -> i32 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_i64(self) -> i64 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    #[must_use]
    pub const fn as_i128(self) -> i128 {
        let (_, lo) = self.0.into_words();
        lo as _
    }

    /// Returns a copy of the number as big endian bytes.
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn to_be_bytes(self) -> [u8; 32] {
        self.0.to_be_bytes()
    }

    /// Returns a copy of the number as little endian bytes.
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn to_le_bytes(self) -> [u8; 32] {
        self.0.to_le_bytes()
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        let words = (self.0).0;
        words[0] == 0 && words[1] == 0
    }




    pub fn checked_add(self, other: Self) -> Result<Self, OverflowError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Add, self, other))
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, OverflowError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Sub, self, other))
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, OverflowError> {
        self.0
            .checked_mul(other.0)
            .map(Self)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Mul, self, other))
    }

    pub fn checked_pow(self, exp: u32) -> Result<Self, OverflowError> {
        self.0
            .checked_pow(exp.into())
            .map(Self)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Pow, self, exp))
    }

    pub fn checked_div(self, other: Self) -> Result<Self, DivideByZeroError> {
        self.0
            .checked_div(other.0)
            .map(Self)
            .ok_or_else(|| DivideByZeroError::new(self))
    }

    pub fn checked_div_euclid(self, other: Self) -> Result<Self, DivideByZeroError> {
        self.checked_div(other)
    }

    pub fn checked_rem(self, other: Self) -> Result<Self, DivideByZeroError> {
        self.0
            .checked_rem(other.0)
            .map(Self)
            .ok_or_else(|| DivideByZeroError::new(self))
    }

    pub fn checked_shr(self, other: u32) -> Result<Self, OverflowError> {
        if other >= 256 {
            return Err(OverflowError::new(OverflowOperation::Shr, self, other));
        }

        Ok(Self(self.0.shr(other)))
    }

    pub fn checked_shl(self, other: u32) -> Result<Self, OverflowError> {
        if other >= 256 {
            return Err(OverflowError::new(OverflowOperation::Shl, self, other));
        }

        Ok(Self(self.0.shl(other)))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn wrapping_add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn wrapping_sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn wrapping_mul(self, other: Self) -> Self {
        Self(self.0.wrapping_mul(other.0))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn wrapping_pow(self, other: u32) -> Self {
        Self(self.0.wrapping_pow(other))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn wrapping_shl(self, other: u32) -> Self {
        Self(self.0.wrapping_shl(other))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    #[inline]
    pub fn wrapping_shr(self, other: u32) -> Self {
        Self(self.0.wrapping_shr(other))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn saturating_mul(self, other: Self) -> Self {
        Self(self.0.saturating_mul(other.0))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn saturating_pow(self, exp: u32) -> Self {
        match self.checked_pow(exp) {
            Ok(value) => value,
            Err(_) => Self::MAX,
        }
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn abs_diff(self, other: Self) -> Self {
        if self < other {
            other - self
        } else {
            self - other
        }
    }

}

impl From<Uint128> for U256 {
    fn from(val: Uint128) -> Self {
        val.u128().into()
    }
}

impl From<Uint64> for U256 {
    fn from(val: Uint64) -> Self {
        val.u64().into()
    }
}

impl From<u128> for U256 {
    fn from(val: u128) -> Self {
        U256(val.into())
    }
}

impl From<u64> for U256 {
    fn from(val: u64) -> Self {
        U256(val.into())
    }
}

impl From<u32> for U256 {
    fn from(val: u32) -> Self {
        U256(val.into())
    }
}

impl From<u16> for U256 {
    fn from(val: u16) -> Self {
        U256(val.into())
    }
}

impl From<u8> for U256 {
    fn from(val: u8) -> Self {
        U256(val.into())
    }
}

impl TryFrom<U256> for Uint128 {
    type Error = ConversionOverflowError;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        Ok(Uint128::new(value.0.try_into().map_err(|_| {
            ConversionOverflowError::new("U256", "Uint128", value.to_string())
        })?))
    }
}

impl TryFrom<I256> for U256 {
    type Error = ConversionOverflowError;

    fn try_from(value: I256) -> Result<Self, Self::Error> {
        if value < I256::zero() {
            return Err(ConversionOverflowError::new("I256", "U256", value.to_string()))
        }

        Ok(value.as_u256())
    }
}

impl TryFrom<&str> for U256 {
    type Error = StdError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        Self::from_str(val)
    }
}

impl FromStr for U256 {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(StdError::generic_err("Parsing u256: received empty string"));
        }

        match BaseU256::from_str(s) {
            Ok(u) => Ok(U256(u)),
            Err(e) => Err(StdError::generic_err(format!("Parsing u256: {}", e))),
        }
    }
}

impl From<U256> for String {
    fn from(original: U256) -> Self {
        original.to_string()
    }
}

impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // The inner type doesn't work as expected with padding, so we
        // work around that.
        let unpadded = self.0.to_string();

        f.pad_integral(true, "", &unpadded)
    }
}

impl Add<U256> for U256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_add(rhs.0)
                .expect("attempt to add with overflow"),
        )
    }
}

impl<'a> Add<&'a U256> for U256 {
    type Output = Self;

    fn add(self, rhs: &'a U256) -> Self {
        self + *rhs
    }
}

impl Sub<U256> for U256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("attempt to subtract with overflow"),
        )
    }
}
forward_ref_binop!(impl Sub, sub for U256, U256);

impl SubAssign<U256> for U256 {
    fn sub_assign(&mut self, rhs: U256) {
        *self = *self - rhs;
    }
}
forward_ref_op_assign!(impl SubAssign, sub_assign for U256, U256);

impl Div<U256> for U256 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .checked_div(rhs.0)
                .expect("attempt to divide by zero"),
        )
    }
}

impl<'a> Div<&'a U256> for U256 {
    type Output = Self;

    fn div(self, rhs: &'a U256) -> Self::Output {
        self / *rhs
    }
}

impl Rem for U256 {
    type Output = Self;

    /// # Panics
    ///
    /// This operation will panic if `rhs` is zero.
    #[inline]
    fn rem(self, rhs: Self) -> Self {
        Self(self.0.rem(rhs.0))
    }
}
forward_ref_binop!(impl Rem, rem for U256, U256);

impl RemAssign<U256> for U256 {
    fn rem_assign(&mut self, rhs: U256) {
        *self = *self % rhs;
    }
}
forward_ref_op_assign!(impl RemAssign, rem_assign for U256, U256);

impl Mul<U256> for U256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .checked_mul(rhs.0)
                .expect("attempt to multiply with overflow"),
        )
    }
}
forward_ref_binop!(impl Mul, mul for U256, U256);

impl MulAssign<U256> for U256 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
forward_ref_op_assign!(impl MulAssign, mul_assign for U256, U256);

impl Shr<u32> for U256 {
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        self.checked_shr(rhs).unwrap_or_else(|_| {
            panic!(
                "right shift error: {} is larger or equal than the number of bits in U256",
                rhs,
            )
        })
    }
}

impl<'a> Shr<&'a u32> for U256 {
    type Output = Self;

    fn shr(self, rhs: &'a u32) -> Self::Output {
        self.shr(*rhs)
    }
}

impl Shl<u32> for U256 {
    type Output = Self;

    fn shl(self, rhs: u32) -> Self::Output {
        self.checked_shl(rhs).unwrap_or_else(|_| {
            panic!(
                "left shift error: {} is larger or equal than the number of bits in U256",
                rhs,
            )
        })
    }
}

impl<'a> Shl<&'a u32> for U256 {
    type Output = Self;

    fn shl(self, rhs: &'a u32) -> Self::Output {
        self.shl(*rhs)
    }
}


impl BitOr for U256 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0.bitor(rhs.0))
    }
}

impl<'a> BitOr<&'a U256> for U256 {
    type Output = Self;

    fn bitor(self, rhs: &'a U256) -> Self::Output {
        self.bitor(*rhs)
    }
}

impl BitAnd for U256 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0.bitand(rhs.0))
    }
}

impl<'a> BitAnd<&'a U256> for U256 {
    type Output = Self;

    fn bitand(self, rhs: &'a U256) -> Self::Output {
        self.bitand(*rhs)
    }
}

impl AddAssign<U256> for U256 {
    fn add_assign(&mut self, rhs: U256) {
        *self = *self + rhs;
    }
}

impl<'a> AddAssign<&'a U256> for U256 {
    fn add_assign(&mut self, rhs: &'a U256) {
        *self = *self + rhs;
    }
}

impl DivAssign<U256> for U256 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<'a> DivAssign<&'a U256> for U256 {
    fn div_assign(&mut self, rhs: &'a U256) {
        *self = *self / rhs;
    }
}

impl ShrAssign<u32> for U256 {
    fn shr_assign(&mut self, rhs: u32) {
        *self = Shr::<u32>::shr(*self, rhs);
    }
}

impl<'a> ShrAssign<&'a u32> for U256 {
    fn shr_assign(&mut self, rhs: &'a u32) {
        *self = Shr::<u32>::shr(*self, *rhs);
    }
}

impl BitOrAssign for U256 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0.bitor_assign(rhs.0)
    }
}

impl BitAndAssign for U256 {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0)
    }
}

impl Serialize for U256 {
    /// Serializes as an integer string using base 10
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for U256 {
    /// Deserialized from an integer string using base 10
    fn deserialize<D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(U256Visitor)
    }
}

struct U256Visitor;

impl<'de> de::Visitor<'de> for U256Visitor {
    type Value = U256;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string-encoded integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        U256::try_from(v).map_err(|e| E::custom(format!("invalid U256 '{}' - {}", v, e)))
    }
}

impl<A> std::iter::Sum<A> for U256
where
    Self: Add<A, Output = Self>,
{
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self {
        iter.fold(Self::zero(), Add::add)
    }
}


impl AsI256 for U256 {
    fn as_i256(self) -> I256 {
        let (hi, lo) = self.0.into_words();
        I256::from_words(hi as _, lo as _)
    }
}

impl AsU256 for bool {
    fn as_u256(self) -> U256 {
        match self {
            true => U256::one(),
            false => U256::zero()
        }
    }
}
