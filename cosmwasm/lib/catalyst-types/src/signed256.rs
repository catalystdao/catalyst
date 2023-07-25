use bnum::prelude::As;
use forward_ref::{forward_ref_binop, forward_ref_op_assign};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Div, DivAssign, Mul, MulAssign, Neg,
    Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign
};
use std::str::FromStr;

use cosmwasm_std::{ConversionOverflowError, DivideByZeroError, forward_ref_partial_eq, Int64, Int128, OverflowError, OverflowOperation, StdError, Uint64, Uint128};


// NOTE: This wrapper is based on CosmWasm's Int256.

// NOTE: There are some inconsistencies between the U256 and the I256 (this file) implementations.
// These have been carried over from the CosmWasm's Uint256 and Int256 wrappers as to keep this
// custom implementation of the types as close as possible to the CosmWasm one (cosmwasm_std 1.3.0).


/// Used internally - we don't want to leak this type since we might change
/// the implementation in the future.
use bnum::types::{I256 as BaseI256, U256 as BaseU256};

use crate::U256;
use crate::errors::DivisionError;
use crate::traits::AsI256;

/// An implementation of i256 that is using strings for JSON encoding/decoding,
/// such that the full i256 range can be used for clients that convert JSON numbers to floats,
/// like JavaScript and jq.
///
/// # Examples
///
/// Use `from` to create instances out of primitive uint types or `new` to provide big
/// endian bytes:
///
/// ```
/// # use catalyst_types::I256;
/// let a = I256::from(258u128);
/// let b = I256::new([
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 2u8,
/// ]);
/// assert_eq!(a, b);
/// ```
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct I256(#[schemars(with = "String")] pub(crate) BaseI256);

forward_ref_partial_eq!(I256, I256);

impl I256 {
    pub const MAX: I256 = I256(BaseI256::MAX);
    pub const MIN: I256 = I256(BaseI256::MIN);

    /// Creates a I256(value) from a big endian representation. It's just an alias for
    /// `from_be_bytes`.
    #[inline]
    pub const fn new(value: [u8; 32]) -> Self {
        Self::from_be_bytes(value)
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
    pub const fn from_be_bytes(data: [u8; 32]) -> Self {
        let words: [u64; 4] = [
            u64::from_le_bytes([
                data[31], data[30], data[29], data[28], data[27], data[26], data[25], data[24],
            ]),
            u64::from_le_bytes([
                data[23], data[22], data[21], data[20], data[19], data[18], data[17], data[16],
            ]),
            u64::from_le_bytes([
                data[15], data[14], data[13], data[12], data[11], data[10], data[9], data[8],
            ]),
            u64::from_le_bytes([
                data[7], data[6], data[5], data[4], data[3], data[2], data[1], data[0],
            ]),
        ];
        Self(BaseI256::from_bits(BaseU256::from_digits(words)))
    }

    #[must_use]
    pub const fn from_le_bytes(data: [u8; 32]) -> Self {
        let words: [u64; 4] = [
            u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]),
            u64::from_le_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]),
            u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
            u64::from_le_bytes([
                data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
            ]),
        ];
        Self(BaseI256::from_bits(BaseU256::from_digits(words)))
    }

    /// Returns a copy of the number as big endian bytes.
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub const fn to_be_bytes(self) -> [u8; 32] {
        let bits = self.0.to_bits();
        let words = bits.digits();
        let words = [
            words[3].to_be_bytes(),
            words[2].to_be_bytes(),
            words[1].to_be_bytes(),
            words[0].to_be_bytes(),
        ];
        unsafe { std::mem::transmute::<[[u8; 8]; 4], [u8; 32]>(words) }
    }

    /// Returns a copy of the number as little endian bytes.
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub const fn to_le_bytes(self) -> [u8; 32] {
        let bits = self.0.to_bits();
        let words = bits.digits();
        let words = [
            words[0].to_le_bytes(),
            words[1].to_le_bytes(),
            words[2].to_le_bytes(),
            words[3].to_le_bytes(),
        ];
        unsafe { std::mem::transmute::<[[u8; 8]; 4], [u8; 32]>(words) }
    }

    #[must_use]
    pub fn as_u8(self) -> u8 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_u16(self) -> u16 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_u128(self) -> u128 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_uint128(self) -> Uint128 {
        Uint128::new(self.0.as_())
    }

    #[must_use]
    pub fn as_u256(self) -> U256 {
        U256(self.0.as_())
    }

    #[must_use]
    pub fn as_i8(self) -> i8 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_i16(self) -> i16 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_i32(self) -> i32 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_i64(self) -> i64 {
        self.0.as_()
    }

    #[must_use]
    pub fn as_i128(self) -> i128 {
        self.0.as_()
    }

    #[must_use]
    pub const fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn pow(self, exp: u32) -> Self {
        Self(self.0.pow(exp))
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

    pub fn checked_div(self, other: Self) -> Result<Self, DivisionError> {
        if other.is_zero() {
            return Err(DivisionError::DivideByZero);
        }
        self.0
            .checked_div(other.0)
            .map(Self)
            .ok_or(DivisionError::Overflow)
    }

    pub fn checked_div_euclid(self, other: Self) -> Result<Self, DivisionError> {
        if other.is_zero() {
            return Err(DivisionError::DivideByZero);
        }
        self.0
            .checked_div_euclid(other.0)
            .map(Self)
            .ok_or(DivisionError::Overflow)
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
            .ok_or_else(|| OverflowError::new(OverflowOperation::Mul, self, I256::parse_str_radix("-1", 10)))
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
    pub fn saturating_pow(self, exp: u32) -> Self {
        Self(self.0.saturating_pow(exp))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn abs_diff(self, other: Self) -> U256 {
        U256(self.0.abs_diff(other.0))
    }

    pub const fn parse_str_radix(src: &str, radix: u32) -> Self {
        Self(BaseI256::parse_str_radix(src, radix))
    }

}

impl From<Uint128> for I256 {
    fn from(val: Uint128) -> Self {
        val.u128().into()
    }
}

impl From<Uint64> for I256 {
    fn from(val: Uint64) -> Self {
        val.u64().into()
    }
}

impl From<u128> for I256 {
    fn from(val: u128) -> Self {
        I256(val.into())
    }
}

impl From<u64> for I256 {
    fn from(val: u64) -> Self {
        I256(val.into())
    }
}

impl From<u32> for I256 {
    fn from(val: u32) -> Self {
        I256(val.into())
    }
}

impl From<u16> for I256 {
    fn from(val: u16) -> Self {
        I256(val.into())
    }
}

impl From<u8> for I256 {
    fn from(val: u8) -> Self {
        I256(val.into())
    }
}

impl From<Int128> for I256 {
    fn from(val: Int128) -> Self {
        val.i128().into()
    }
}

impl From<Int64> for I256 {
    fn from(val: Int64) -> Self {
        val.i64().into()
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
        Ok(Uint128::new(value.0.try_into().map_err(|_| {
            ConversionOverflowError::new("I256", "Uint128", value.to_string())
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

impl TryFrom<&str> for I256 {
    type Error = StdError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        Self::from_str(val)
    }
}

impl FromStr for I256 {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match BaseI256::from_str_radix(s, 10) {
            Ok(u) => Ok(Self(u)),
            Err(e) => Err(StdError::generic_err(format!("Parsing I256: {}", e))),
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
        // The inner type doesn't work as expected with padding, so we
        // work around that. Remove this code when the upstream padding is fixed.
        let unpadded = self.0.to_string();
        let numeric = unpadded.strip_prefix('-').unwrap_or(&unpadded);

        f.pad_integral(self >= &Self::zero(), "", numeric)
    }
}

impl Add<I256> for I256 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        I256(self.0.checked_add(rhs.0).unwrap())
    }
}
forward_ref_binop!(impl Add, add for I256, I256);

impl Sub<I256> for I256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        I256(self.0.checked_sub(rhs.0).unwrap())
    }
}
forward_ref_binop!(impl Sub, sub for I256, I256);

impl SubAssign<I256> for I256 {
    fn sub_assign(&mut self, rhs: I256) {
        self.0 = self.0.checked_sub(rhs.0).unwrap();
    }
}
forward_ref_op_assign!(impl SubAssign, sub_assign for I256, I256);

impl Div<I256> for I256 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0.checked_div(rhs.0).unwrap())
    }
}
forward_ref_binop!(impl Div, div for I256, I256);

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

impl Not for I256 {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl Neg for I256 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl RemAssign<I256> for I256 {
    fn rem_assign(&mut self, rhs: I256) {
        *self = *self % rhs;
    }
}
forward_ref_op_assign!(impl RemAssign, rem_assign for I256, I256);

impl Mul<I256> for I256 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.checked_mul(rhs.0).unwrap())
    }
}
forward_ref_binop!(impl Mul, mul for I256, I256);

impl MulAssign<I256> for I256 {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 = self.0.checked_mul(rhs.0).unwrap();
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
forward_ref_binop!(impl Shr, shr for I256, u32);

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
forward_ref_binop!(impl Shl, shl for I256, u32);

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
        self.0 = self.0.checked_add(rhs.0).unwrap();
    }
}
forward_ref_op_assign!(impl AddAssign, add_assign for I256, I256);

impl DivAssign<I256> for I256 {
    fn div_assign(&mut self, rhs: Self) {
        self.0 = self.0.checked_div(rhs.0).unwrap();
    }
}
forward_ref_op_assign!(impl DivAssign, div_assign for I256, I256);

impl ShrAssign<u32> for I256 {
    fn shr_assign(&mut self, rhs: u32) {
        *self = Shr::<u32>::shr(*self, rhs);
    }
}
forward_ref_op_assign!(impl ShrAssign, shr_assign for I256, u32);

impl ShlAssign<u32> for I256 {
    fn shl_assign(&mut self, rhs: u32) {
        *self = Shl::<u32>::shl(*self, rhs);
    }
}
forward_ref_op_assign!(impl ShlAssign, shl_assign for I256, u32);

impl BitOrAssign for I256 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0.bitor_assign(rhs.0)
    }
}
forward_ref_op_assign!(impl BitOrAssign, bitor_assign for I256, I256);

impl BitAndAssign for I256 {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0)
    }
}
forward_ref_op_assign!(impl BitAndAssign, bitand_assign for I256, I256);

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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{to_vec, from_slice};

    #[test]
    fn size_of_works() {
        assert_eq!(std::mem::size_of::<I256>(), 32);
    }

    #[test]
    fn i256_new_works() {
        let num = I256::new([1; 32]);
        let a: [u8; 32] = num.to_be_bytes();
        assert_eq!(a, [1; 32]);

        let be_bytes = [
            0u8, 222u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 2u8, 3u8,
        ];
        let num = I256::new(be_bytes);
        let resulting_bytes: [u8; 32] = num.to_be_bytes();
        assert_eq!(be_bytes, resulting_bytes);
    }

    #[test]
    fn i256_zero_works() {
        let zero = I256::zero();
        assert_eq!(zero.to_be_bytes(), [0; 32]);
    }

    #[test]
    fn u256_one_works() {
        let one = I256::one();
        let mut one_be = [0; 32];
        one_be[31] = 1;

        assert_eq!(one.to_be_bytes(), one_be);
    }

    #[test]
    fn i256_endianness() {
        let be_bytes = [
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 2u8, 3u8,
        ];
        let le_bytes = [
            3u8, 2u8, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ];

        // These should all be the same.
        let num1 = I256::new(be_bytes);
        let num2 = I256::from_be_bytes(be_bytes);
        let num3 = I256::from_le_bytes(le_bytes);
        assert_eq!(num1, I256::from(65536u32 + 512 + 3));
        assert_eq!(num1, num2);
        assert_eq!(num1, num3);
    }

    #[test]
    fn i256_convert_from() {
        let a = I256::from(5u128);
        assert_eq!(a.0, BaseI256::from(5u32));

        let a = I256::from(5u64);
        assert_eq!(a.0, BaseI256::from(5u32));

        let a = I256::from(5u32);
        assert_eq!(a.0, BaseI256::from(5u32));

        let a = I256::from(5u16);
        assert_eq!(a.0, BaseI256::from(5u32));

        let a = I256::from(5u8);
        assert_eq!(a.0, BaseI256::from(5u32));

        let a = I256::from(-5i128);
        assert_eq!(a.0, BaseI256::from(-5i32));

        let a = I256::from(-5i64);
        assert_eq!(a.0, BaseI256::from(-5i32));

        let a = I256::from(-5i32);
        assert_eq!(a.0, BaseI256::from(-5i32));

        let a = I256::from(-5i16);
        assert_eq!(a.0, BaseI256::from(-5i32));

        let a = I256::from(-5i8);
        assert_eq!(a.0, BaseI256::from(-5i32));

        let result = I256::try_from("34567");
        assert_eq!(
            result.unwrap().0,
            BaseI256::from_str_radix("34567", 10).unwrap()
        );

        let result = I256::try_from("1.23");
        assert!(result.is_err());
    }

    #[test]
    fn i256_implements_display() {
        let a = I256::from(12345u32);
        assert_eq!(format!("Embedded: {}", a), "Embedded: 12345");
        assert_eq!(a.to_string(), "12345");

        let a = I256::from(-12345i32);
        assert_eq!(format!("Embedded: {}", a), "Embedded: -12345");
        assert_eq!(a.to_string(), "-12345");

        let a = I256::zero();
        assert_eq!(format!("Embedded: {}", a), "Embedded: 0");
        assert_eq!(a.to_string(), "0");
    }

    #[test]
    fn i256_display_padding_works() {
        let a = I256::from(123u64);
        assert_eq!(format!("Embedded: {:05}", a), "Embedded: 00123");

        let a = I256::from(-123i64);
        assert_eq!(format!("Embedded: {:05}", a), "Embedded: -0123");
    }

    #[test]
    fn i256_to_be_bytes_works() {
        assert_eq!(I256::zero().to_be_bytes(), [0; 32]);

        let mut max = [0xff; 32];
        max[0] = 0x7f;
        assert_eq!(I256::MAX.to_be_bytes(), max);

        let mut one = [0; 32];
        one[31] = 1;
        assert_eq!(I256::from(1u128).to_be_bytes(), one);
        // Python: `[b for b in (240282366920938463463374607431768124608).to_bytes(32, "big")]`
        assert_eq!(
            I256::from(240282366920938463463374607431768124608u128).to_be_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 180, 196, 179, 87, 165, 121, 59,
                133, 246, 117, 221, 191, 255, 254, 172, 192
            ]
        );
        assert_eq!(
            I256::from_be_bytes([
                17, 4, 23, 32, 87, 67, 123, 200, 58, 91, 0, 38, 33, 21, 67, 78, 87, 76, 65, 54,
                211, 201, 192, 7, 42, 233, 2, 240, 200, 115, 150, 240
            ])
            .to_be_bytes(),
            [
                17, 4, 23, 32, 87, 67, 123, 200, 58, 91, 0, 38, 33, 21, 67, 78, 87, 76, 65, 54,
                211, 201, 192, 7, 42, 233, 2, 240, 200, 115, 150, 240
            ]
        );
    }

    #[test]
    fn i256_to_le_bytes_works() {
        assert_eq!(I256::zero().to_le_bytes(), [0; 32]);

        let mut max = [0xff; 32];
        max[31] = 0x7f;
        assert_eq!(I256::MAX.to_le_bytes(), max);

        let mut one = [0; 32];
        one[0] = 1;
        assert_eq!(I256::from(1u128).to_le_bytes(), one);
        // Python: `[b for b in (240282366920938463463374607431768124608).to_bytes(64, "little")]`
        assert_eq!(
            I256::from(240282366920938463463374607431768124608u128).to_le_bytes(),
            [
                192, 172, 254, 255, 191, 221, 117, 246, 133, 59, 121, 165, 87, 179, 196, 180, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]
        );
        assert_eq!(
            I256::from_be_bytes([
                17, 4, 23, 32, 87, 67, 123, 200, 58, 91, 0, 38, 33, 21, 67, 78, 87, 76, 65, 54,
                211, 201, 192, 7, 42, 233, 2, 240, 200, 115, 150, 240
            ])
            .to_le_bytes(),
            [
                240, 150, 115, 200, 240, 2, 233, 42, 7, 192, 201, 211, 54, 65, 76, 87, 78, 67, 21,
                33, 38, 0, 91, 58, 200, 123, 67, 87, 32, 23, 4, 17
            ]
        );
    }

    #[test]
    fn i256_is_zero_works() {
        assert!(I256::zero().is_zero());
        assert!(I256(BaseI256::from(0u32)).is_zero());

        assert!(!I256::from(1u32).is_zero());
        assert!(!I256::from(123u32).is_zero());
        assert!(!I256::from(-123i32).is_zero());
    }

    #[test]
    fn i256_wrapping_methods() {
        // wrapping_add
        assert_eq!(
            I256::from(2u32).wrapping_add(I256::from(2u32)),
            I256::from(4u32)
        ); // non-wrapping
        assert_eq!(I256::MAX.wrapping_add(I256::from(1u32)), I256::MIN); // wrapping

        // wrapping_sub
        assert_eq!(
            I256::from(7u32).wrapping_sub(I256::from(5u32)),
            I256::from(2u32)
        ); // non-wrapping
        assert_eq!(I256::MIN.wrapping_sub(I256::from(1u32)), I256::MAX); // wrapping

        // wrapping_mul
        assert_eq!(
            I256::from(3u32).wrapping_mul(I256::from(2u32)),
            I256::from(6u32)
        ); // non-wrapping
        assert_eq!(
            I256::MAX.wrapping_mul(I256::from(2u32)),
            I256::from(-2i32)
        ); // wrapping

        // wrapping_pow
        assert_eq!(I256::from(2u32).wrapping_pow(3), I256::from(8u32)); // non-wrapping
        assert_eq!(I256::MAX.wrapping_pow(2), I256::from(1u32)); // wrapping
    }

    #[test]
    fn i256_json() {
        let orig = I256::from(1234567890987654321u128);
        let serialized = to_vec(&orig).unwrap();
        assert_eq!(serialized.as_slice(), b"\"1234567890987654321\"");
        let parsed: I256 = from_slice(&serialized).unwrap();
        assert_eq!(parsed, orig);
    }

    #[test]
    fn i256_compare() {
        let a = I256::from(12345u32);
        let b = I256::from(23456u32);

        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, I256::from(12345u32));
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn i256_math() {
        let a = I256::from(-12345i32);
        let b = I256::from(23456u32);

        // test + with owned and reference right hand side
        assert_eq!(a + b, I256::from(11111u32));
        assert_eq!(a + &b, I256::from(11111u32));

        // test - with owned and reference right hand side
        assert_eq!(b - a, I256::from(35801u32));
        assert_eq!(b - &a, I256::from(35801u32));

        // test += with owned and reference right hand side
        let mut c = I256::from(300000u32);
        c += b;
        assert_eq!(c, I256::from(323456u32));
        let mut d = I256::from(300000u32);
        d += &b;
        assert_eq!(d, I256::from(323456u32));

        // test -= with owned and reference right hand side
        let mut c = I256::from(300000u32);
        c -= b;
        assert_eq!(c, I256::from(276544u32));
        let mut d = I256::from(300000u32);
        d -= &b;
        assert_eq!(d, I256::from(276544u32));

        // test - with negative result
        assert_eq!(a - b, I256::from(-35801i32));
    }

    #[test]
    #[should_panic]
    fn i256_add_overflow_panics() {
        let _ = I256::MAX + I256::from(12u32);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn i256_sub_works() {
        assert_eq!(I256::from(2u32) - I256::from(1u32), I256::from(1u32));
        assert_eq!(I256::from(2u32) - I256::from(0u32), I256::from(2u32));
        assert_eq!(I256::from(2u32) - I256::from(2u32), I256::from(0u32));
        assert_eq!(I256::from(2u32) - I256::from(3u32), I256::from(-1i32));

        // works for refs
        let a = I256::from(10u32);
        let b = I256::from(3u32);
        let expected = I256::from(7u32);
        assert_eq!(a - b, expected);
        assert_eq!(a - &b, expected);
        assert_eq!(&a - b, expected);
        assert_eq!(&a - &b, expected);
    }

    #[test]
    #[should_panic]
    fn i256_sub_overflow_panics() {
        let _ = I256::MIN + I256::one() - I256::from(2u32);
    }

    #[test]
    fn i256_sub_assign_works() {
        let mut a = I256::from(14u32);
        a -= I256::from(2u32);
        assert_eq!(a, I256::from(12u32));

        // works for refs
        let mut a = I256::from(10u32);
        let b = I256::from(3u32);
        let expected = I256::from(7u32);
        a -= &b;
        assert_eq!(a, expected);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn i256_mul_works() {
        assert_eq!(I256::from(2u32) * I256::from(3u32), I256::from(6u32));
        assert_eq!(I256::from(2u32) * I256::zero(), I256::zero());

        // works for refs
        let a = I256::from(11u32);
        let b = I256::from(3u32);
        let expected = I256::from(33u32);
        assert_eq!(a * b, expected);
        assert_eq!(a * &b, expected);
        assert_eq!(&a * b, expected);
        assert_eq!(&a * &b, expected);
    }

    #[test]
    fn i256_mul_assign_works() {
        let mut a = I256::from(14u32);
        a *= I256::from(2u32);
        assert_eq!(a, I256::from(28u32));

        // works for refs
        let mut a = I256::from(10u32);
        let b = I256::from(3u32);
        a *= &b;
        assert_eq!(a, I256::from(30u32));
    }

    #[test]
    fn i256_pow_works() {
        assert_eq!(I256::from(2u32).pow(2), I256::from(4u32));
        assert_eq!(I256::from(2u32).pow(10), I256::from(1024u32));
    }

    #[test]
    #[should_panic]
    fn i256_pow_overflow_panics() {
        _ = I256::MAX.pow(2u32);
    }

    #[test]
    fn i256_shr_works() {
        let original = I256::new([
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 2u8, 0u8, 4u8, 2u8,
        ]);

        let shifted = I256::new([
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 128u8, 1u8, 0u8,
        ]);

        assert_eq!(original >> 2u32, shifted);
    }

    #[test]
    #[should_panic]
    fn i256_shr_overflow_panics() {
        let _ = I256::from(1u32) >> 256u32;
    }

    #[test]
    fn sum_works() {
        let nums = vec![
            I256::from(17u32),
            I256::from(123u32),
            I256::from(540u32),
            I256::from(82u32),
        ];
        let expected = I256::from(762u32);

        let sum_as_ref: I256 = nums.iter().sum();
        assert_eq!(expected, sum_as_ref);

        let sum_as_owned: I256 = nums.into_iter().sum();
        assert_eq!(expected, sum_as_owned);
    }

    #[test]
    fn i256_methods() {
        // checked_*
        assert!(matches!(
            I256::MAX.checked_add(I256::from(1u32)),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            I256::from(1u32).checked_add(I256::from(1u32)),
            Ok(I256::from(2u32)),
        );
        assert!(matches!(
            I256::MIN.checked_sub(I256::from(1u32)),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            I256::from(2u32).checked_sub(I256::from(1u32)),
            Ok(I256::from(1u32)),
        );
        assert!(matches!(
            I256::MAX.checked_mul(I256::from(2u32)),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            I256::from(2u32).checked_mul(I256::from(2u32)),
            Ok(I256::from(4u32)),
        );
        assert!(matches!(
            I256::MAX.checked_pow(2u32),
            Err(OverflowError { .. })
        ));
        assert_eq!(I256::from(2u32).checked_pow(3u32), Ok(I256::from(8u32)),);
        assert_eq!(
            I256::MAX.checked_div(I256::from(0u32)),
            Err(DivisionError::DivideByZero)
        );
        assert_eq!(
            I256::from(6u32).checked_div(I256::from(2u32)),
            Ok(I256::from(3u32)),
        );
        assert_eq!(
            I256::MAX.checked_div_euclid(I256::from(0u32)),
            Err(DivisionError::DivideByZero)
        );
        assert_eq!(
            I256::from(6u32).checked_div_euclid(I256::from(2u32)),
            Ok(I256::from(3u32)),
        );
        assert_eq!(
            I256::from(7u32).checked_div_euclid(I256::from(2u32)),
            Ok(I256::from(3u32)),
        );
        assert!(matches!(
            I256::MAX.checked_rem(I256::from(0u32)),
            Err(DivideByZeroError { .. })
        ));
        // checked_* with negative numbers
        assert_eq!(
            I256::from(-12i32).checked_div(I256::from(10i32)),
            Ok(I256::from(-1i32)),
        );
        assert_eq!(
            I256::from(-2i32).checked_pow(3u32),
            Ok(I256::from(-8i32)),
        );
        assert_eq!(
            I256::from(-6i32).checked_mul(I256::from(-7i32)),
            Ok(I256::from(42i32)),
        );
        assert_eq!(
            I256::from(-2i32).checked_add(I256::from(3i32)),
            Ok(I256::from(1i32)),
        );
        assert_eq!(
            I256::from(-1i32).checked_div_euclid(I256::from(-2i32)),
            Ok(I256::from(1u32)),
        );

        // saturating_*
        assert_eq!(I256::MAX.saturating_add(I256::from(1u32)), I256::MAX);
        assert_eq!(I256::MIN.saturating_sub(I256::from(1u32)), I256::MIN);
        assert_eq!(I256::MAX.saturating_mul(I256::from(2u32)), I256::MAX);
        assert_eq!(I256::from(4u32).saturating_pow(2u32), I256::from(16u32));
        assert_eq!(I256::MAX.saturating_pow(2u32), I256::MAX);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn i256_implements_rem() {
        let a = I256::from(10u32);
        assert_eq!(a % I256::from(10u32), I256::zero());
        assert_eq!(a % I256::from(2u32), I256::zero());
        assert_eq!(a % I256::from(1u32), I256::zero());
        assert_eq!(a % I256::from(3u32), I256::from(1u32));
        assert_eq!(a % I256::from(4u32), I256::from(2u32));

        assert_eq!(
            I256::from(-12i32) % I256::from(10i32),
            I256::from(-2i32)
        );
        assert_eq!(
            I256::from(12i32) % I256::from(-10i32),
            I256::from(2i32)
        );
        assert_eq!(
            I256::from(-12i32) % I256::from(-10i32),
            I256::from(-2i32)
        );

        // works for refs
        let a = I256::from(10u32);
        let b = I256::from(3u32);
        let expected = I256::from(1u32);
        assert_eq!(a % b, expected);
        assert_eq!(a % &b, expected);
        assert_eq!(&a % b, expected);
        assert_eq!(&a % &b, expected);
    }

    #[test]
    #[should_panic(expected = "divisor of zero")]
    fn i256_rem_panics_for_zero() {
        let _ = I256::from(10u32) % I256::zero();
    }

    #[test]
    fn i256_rem_assign_works() {
        let mut a = I256::from(30u32);
        a %= I256::from(4u32);
        assert_eq!(a, I256::from(2u32));

        // works for refs
        let mut a = I256::from(25u32);
        let b = I256::from(6u32);
        a %= &b;
        assert_eq!(a, I256::from(1u32));
    }

    #[test]
    fn i256_shr() {
        let x: I256 = 0x8000_0000_0000_0000_0000_0000_0000_0000u128.into();
        assert_eq!(x >> 0, x); // right shift by 0 should be no-op
        assert_eq!(
            x >> 1,
            I256::from(0x4000_0000_0000_0000_0000_0000_0000_0000u128)
        );
        assert_eq!(
            x >> 4,
            I256::from(0x0800_0000_0000_0000_0000_0000_0000_0000u128)
        );
        // right shift of MIN value by the maximum shift value should result in -1 (filled with 1s)
        assert_eq!(
            I256::MIN >> (std::mem::size_of::<I256>() as u32 * 8 - 1),
            -I256::one()
        );
    }

    #[test]
    fn i256_shl() {
        let x: I256 = 0x0800_0000_0000_0000_0000_0000_0000_0000u128.into();
        assert_eq!(x << 0, x); // left shift by 0 should be no-op
        assert_eq!(
            x << 1,
            I256::from(0x1000_0000_0000_0000_0000_0000_0000_0000u128)
        );
        assert_eq!(
            x << 4,
            I256::from(0x8000_0000_0000_0000_0000_0000_0000_0000u128)
        );
        // left shift by by the maximum shift value should result in MIN
        assert_eq!(
            I256::one() << (std::mem::size_of::<I256>() as u32 * 8 - 1),
            I256::MIN
        );
    }

    #[test]
    fn i256_abs_diff_works() {
        let a = I256::from(42u32);
        let b = I256::from(5u32);
        let expected = U256::from(37u32);
        assert_eq!(a.abs_diff(b), expected);
        assert_eq!(b.abs_diff(a), expected);

        let c = I256::from(-5i32);
        assert_eq!(b.abs_diff(c), U256::from(10u32));
        assert_eq!(c.abs_diff(b), U256::from(10u32));
    }

    #[test]
    #[should_panic = "attempt to negate with overflow"]
    fn i256_neg_min_panics() {
        _ = -I256::MIN;
    }

    #[test]
    fn i256_partial_eq() {
        let test_cases = [(1, 1, true), (42, 42, true), (42, 24, false), (0, 0, true)]
            .into_iter()
            .map(|(lhs, rhs, expected): (u64, u64, bool)| {
                (I256::from(lhs), I256::from(rhs), expected)
            });

        #[allow(clippy::op_ref)]
        for (lhs, rhs, expected) in test_cases {
            assert_eq!(lhs == rhs, expected);
            assert_eq!(&lhs == rhs, expected);
            assert_eq!(lhs == &rhs, expected);
            assert_eq!(&lhs == &rhs, expected);
        }
    }
}
