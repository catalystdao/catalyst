use forward_ref::{forward_ref_binop, forward_ref_op_assign};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, Shr, ShrAssign, Sub, SubAssign, BitOr, BitOrAssign, BitAndAssign, BitAnd, Neg};
use std::str::FromStr;

use cosmwasm_std::{forward_ref_partial_eq, StdError, Uint128, OverflowError, OverflowOperation, DivideByZeroError, Uint64, ConversionOverflowError};


// NOTE: This wrapper is based on CosmWasm's Uint256.


/// Used internally - we don't want to leak this type since we might change
/// the implementation in the future.
use ethnum::{I256 as BaseI256, int};

use crate::U256;
use crate::traits::AsI256;

/// An implementation of i256 that is using strings for JSON encoding/decoding,
/// such that the full i256 range can be used for clients that convert JSON numbers to floats,
/// like JavaScript and jq.
///
/// This implementation is based on CosmWasm's Uint implementations
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct I256(#[schemars(with = "String")] BaseI256);

forward_ref_partial_eq!(I256, I256);

impl I256 {
    pub const MAX: I256 = I256(BaseI256::MAX);
    pub const MIN: I256 = I256(BaseI256::MIN);

    /// Create a I256 from two i128 values.
    /// 
    /// **NOTE**: this method is provided for maximum efficiency when creating the base structure (ethnum::I256).
    ///
    pub const fn from_words(hi: i128, lo: i128) -> Self {
        Self(BaseI256::from_words(hi, lo))
    }

    /// Decompose a I256 into two i128 values.
    /// 
    /// **NOTE**: this method is provided for maximum efficiency when decomposing the base structure (ethnum::I256).
    ///
    pub const fn into_words(self) -> (i128, i128) {
        self.0.into_words()
    }

    /// Creates an I256(0)
    #[inline]
    pub const fn zero() -> Self {
        Self(BaseI256::ZERO)
    }

    /// Creates an I256(1)
    #[inline]
    pub const fn one() -> Self {
        Self(BaseI256::ONE)
    }

    #[must_use]
    pub fn from_be_bytes(data: [u8; 32]) -> Self {
        Self(BaseI256::from_be_bytes(data))
    }

    #[must_use]
    pub fn from_le_bytes(data: [u8; 32]) -> Self {
        Self(BaseI256::from_le_bytes(data))
    }

    /// A conversion from `i128` that, unlike the one provided by the `From` trait,
    /// can be used in a `const` context.
    #[must_use]
    pub const fn from_i128(num: i128) -> Self {
        Self(BaseI256::from_words(0, num))
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
        lo as _
    }

    #[must_use]
    pub const fn as_uint128(self) -> Uint128 {
        Uint128::new(self.0.as_u128())
    }

    #[must_use]
    pub const fn as_u256(self) -> U256 {
        let (hi, lo) = self.0.into_words();
        U256::from_words(hi as _, lo as _)
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
        lo
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
            .checked_pow(exp)
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

    pub fn checked_neg(self) -> Result<Self, OverflowError> {
        self.0
            .checked_neg()
            .map(Self)
            .ok_or_else(|| OverflowError::new(OverflowOperation::Mul, self, I256(int!("-1"))))
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
    #[inline]
    pub fn wrapping_neg(self) -> Self {
        Self(self.0.wrapping_neg())
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
    pub fn abs_diff(self, other: Self) -> U256 {
        if self < other {
            other.wrapping_sub(self).as_u256()
        } else {
            self.wrapping_sub(other).as_u256()
        }
    }

}

impl From<Uint128> for I256 {
    fn from(val: Uint128) -> Self {
        I256(val.u128().into())
    }
}

impl From<Uint64> for I256 {
    fn from(val: Uint64) -> Self {
        I256(val.u64().into())
    }
}

impl From<i128> for I256 {
    fn from(val: i128) -> Self {
        I256(val.into())
    }
}

impl From<i64> for I256 {
    fn from(val: i64) -> Self {
        I256(val.into())
    }
}

impl From<i32> for I256 {
    fn from(val: i32) -> Self {
        I256(val.into())
    }
}

impl From<i16> for I256 {
    fn from(val: i16) -> Self {
        I256(val.into())
    }
}

impl From<i8> for I256 {
    fn from(val: i8) -> Self {
        I256(val.into())
    }
}

impl TryFrom<I256> for Uint128 {
    type Error = ConversionOverflowError;

    fn try_from(value: I256) -> Result<Self, Self::Error> {
        // ! As of ethnum 1.3.2, 'I256::try_into<u128>' overflows silently for negative values of I256
        // ! A workaround has been implemented.
        if value < I256::zero() {
            return Err(ConversionOverflowError::new("I256", "Uint128", value.to_string()))
        }

        Ok(Uint128::new(value.0.as_u256().try_into().map_err(|_| {
            ConversionOverflowError::new("I256", "Uint128", value.to_string())
        })?))
    }
}

impl TryFrom<U256> for I256 {
    type Error = ConversionOverflowError;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        if value > I256::MAX.as_u256() {
            return Err(ConversionOverflowError::new("U256", "I256", value.to_string()))
        }

        Ok(value.as_i256())
    }
}

impl TryFrom<&str> for I256 {
    type Error = StdError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        Self::from_str(val)
    }
}

impl FromStr for I256 {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(StdError::generic_err("Parsing i256: received empty string"));
        }

        match BaseI256::from_str(s) {
            Ok(u) => Ok(I256(u)),
            Err(e) => Err(StdError::generic_err(format!("Parsing i256: {}", e))),
        }
    }
}

impl From<I256> for String {
    fn from(original: I256) -> Self {
        original.to_string()
    }
}

impl fmt::Display for I256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let unpadded = self.0.to_string();

        f.pad_integral(true, "", &unpadded)
    }
}

impl Add<I256> for I256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_add(rhs.0)
                .expect("attempt to add with overflow"),
        )
    }
}

impl<'a> Add<&'a I256> for I256 {
    type Output = Self;

    fn add(self, rhs: &'a I256) -> Self {
        self + *rhs
    }
}

impl Sub<I256> for I256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("attempt to subtract with overflow"),
        )
    }
}
forward_ref_binop!(impl Sub, sub for I256, I256);

impl SubAssign<I256> for I256 {
    fn sub_assign(&mut self, rhs: I256) {
        *self = *self - rhs;
    }
}
forward_ref_op_assign!(impl SubAssign, sub_assign for I256, I256);

impl Div<I256> for I256 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .checked_div(rhs.0)
                .expect("attempt to divide by zero"),
        )
    }
}

impl<'a> Div<&'a I256> for I256 {
    type Output = Self;

    fn div(self, rhs: &'a I256) -> Self::Output {
        self / *rhs
    }
}

impl Rem for I256 {
    type Output = Self;

    /// # Panics
    ///
    /// This operation will panic if `rhs` is zero.
    #[inline]
    fn rem(self, rhs: Self) -> Self {
        Self(self.0.rem(rhs.0))
    }
}
forward_ref_binop!(impl Rem, rem for I256, I256);

impl RemAssign<I256> for I256 {
    fn rem_assign(&mut self, rhs: I256) {
        *self = *self % rhs;
    }
}
forward_ref_op_assign!(impl RemAssign, rem_assign for I256, I256);

impl Mul<I256> for I256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .checked_mul(rhs.0)
                .expect("attempt to multiply with overflow"),
        )
    }
}
forward_ref_binop!(impl Mul, mul for I256, I256);

impl MulAssign<I256> for I256 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
forward_ref_op_assign!(impl MulAssign, mul_assign for I256, I256);

impl Shr<u32> for I256 {
    type Output = Self;

    fn shr(self, rhs: u32) -> Self::Output {
        self.checked_shr(rhs).unwrap_or_else(|_| {
            panic!(
                "right shift error: {} is larger or equal than the number of bits in I256",
                rhs,
            )
        })
    }
}

impl<'a> Shr<&'a u32> for I256 {
    type Output = Self;

    fn shr(self, rhs: &'a u32) -> Self::Output {
        self.shr(*rhs)
    }
}

impl Shl<u32> for I256 {
    type Output = Self;

    fn shl(self, rhs: u32) -> Self::Output {
        self.checked_shl(rhs).unwrap_or_else(|_| {
            panic!(
                "left shift error: {} is larger or equal than the number of bits in I256",
                rhs,
            )
        })
    }
}

impl<'a> Shl<&'a u32> for I256 {
    type Output = Self;

    fn shl(self, rhs: &'a u32) -> Self::Output {
        self.shl(*rhs)
    }
}

impl Neg for I256 {
    type Output = I256;

    fn neg(self) -> Self::Output {
        self.checked_neg().unwrap_or_else(|_| {
            panic!(
                "negation overflow: negated {} is larger than I256::MAX",
                self
            )
        })
    }
}


impl BitOr for I256 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0.bitor(rhs.0))
    }
}

impl<'a> BitOr<&'a I256> for I256 {
    type Output = Self;

    fn bitor(self, rhs: &'a I256) -> Self::Output {
        self.bitor(*rhs)
    }
}

impl BitAnd for I256 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0.bitand(rhs.0))
    }
}

impl<'a> BitAnd<&'a I256> for I256 {
    type Output = Self;

    fn bitand(self, rhs: &'a I256) -> Self::Output {
        self.bitand(*rhs)
    }
}

impl AddAssign<I256> for I256 {
    fn add_assign(&mut self, rhs: I256) {
        *self = *self + rhs;
    }
}

impl<'a> AddAssign<&'a I256> for I256 {
    fn add_assign(&mut self, rhs: &'a I256) {
        *self = *self + rhs;
    }
}

impl DivAssign<I256> for I256 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl<'a> DivAssign<&'a I256> for I256 {
    fn div_assign(&mut self, rhs: &'a I256) {
        *self = *self / rhs;
    }
}

impl ShrAssign<u32> for I256 {
    fn shr_assign(&mut self, rhs: u32) {
        *self = Shr::<u32>::shr(*self, rhs);
    }
}

impl<'a> ShrAssign<&'a u32> for I256 {
    fn shr_assign(&mut self, rhs: &'a u32) {
        *self = Shr::<u32>::shr(*self, *rhs);
    }
}

impl BitOrAssign for I256 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0.bitor_assign(rhs.0)
    }
}

impl BitAndAssign for I256 {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0)
    }
}

impl Serialize for I256 {
    /// Serializes as an integer string using base 10
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for I256 {
    /// Deserialized from an integer string using base 10
    fn deserialize<D>(deserializer: D) -> Result<I256, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(I256Visitor)
    }
}

struct I256Visitor;

impl<'de> de::Visitor<'de> for I256Visitor {
    type Value = I256;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string-encoded integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        I256::try_from(v).map_err(|e| E::custom(format!("invalid I256 '{}' - {}", v, e)))
    }
}

impl<A> std::iter::Sum<A> for I256
where
    Self: Add<A, Output = Self>,
{
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self {
        iter.fold(Self::zero(), Add::add)
    }
}

impl AsI256 for bool {
    fn as_i256(self) -> I256 {
        match self {
            true => I256::one(),
            false => I256::zero()
        }
    }
}
