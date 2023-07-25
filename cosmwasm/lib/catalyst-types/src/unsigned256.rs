use bnum::prelude::As;
use forward_ref::{forward_ref_binop, forward_ref_op_assign};
use schemars::JsonSchema;
use serde::{de, ser, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Div, DivAssign, Mul, MulAssign,
    Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign
};
use std::str::FromStr;

use cosmwasm_std::{ConversionOverflowError, DivideByZeroError, forward_ref_partial_eq, OverflowError, OverflowOperation, StdError, Uint64, Uint128};


// NOTE: This wrapper is based on CosmWasm's Uint256.

// NOTE: There are some inconsistencies between the U256 (this file) and the I256 implementations.
// These have been carried over from the CosmWasm's Uint256 and Int256 wrappers as to keep this
// custom implementation of the types as close as possible to the CosmWasm one (cosmwasm_std 1.3.0).


/// Used internally - we don't want to leak this type since we might change
/// the implementation in the future.
use bnum::types::U256 as BaseU256;

use crate::I256;
use crate::traits::AsU256;

/// An implementation of u256 that is using strings for JSON encoding/decoding,
/// such that the full u256 range can be used for clients that convert JSON numbers to floats,
/// like JavaScript and jq.
///
/// # Examples
///
/// Use `from` to create instances out of primitive uint types or `new` to provide big
/// endian bytes:
///
/// ```
/// # use catalyst_types::U256;
/// let a = U256::from(258u128);
/// let b = U256::new([
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
///     0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 2u8,
/// ]);
/// assert_eq!(a, b);
/// ```
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
pub struct U256(#[schemars(with = "String")] pub(crate) BaseU256);

forward_ref_partial_eq!(U256, U256);

impl U256 {
    pub const MAX: U256 = U256(BaseU256::MAX);
    pub const MIN: U256 = U256(BaseU256::ZERO);

    /// Creates a Unt256(value) from a big endian representation. It's just an alias for
    /// `from_be_bytes`.
    #[inline]
    pub const fn new(value: [u8; 32]) -> Self {
        Self::from_be_bytes(value)
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
        Self(BaseU256::from_digits(words))
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
        Self(BaseU256::from_digits(words))
    }

    /// A conversion from `u128` that, unlike the one provided by the `From` trait,
    /// can be used in a `const` context.
    #[must_use]
    pub const fn from_u128(num: u128) -> Self {
        let bytes = num.to_le_bytes();

        Self::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
    }

    /// A conversion from `Uint128` that, unlike the one provided by the `From` trait,
    /// can be used in a `const` context.
    #[must_use]
    pub const fn from_uint128(num: Uint128) -> Self {
        Self::from_u128(num.u128())
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
    pub fn as_i256(self) -> I256 {
        I256(self.0.as_())
    }

    /// Returns a copy of the number as big endian bytes.
    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub const fn to_be_bytes(self) -> [u8; 32] {
        let words = self.0.digits();
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
        let words = self.0.digits();
        let words = [
            words[0].to_le_bytes(),
            words[1].to_le_bytes(),
            words[2].to_le_bytes(),
            words[3].to_le_bytes(),
        ];
        unsafe { std::mem::transmute::<[[u8; 8]; 4], [u8; 32]>(words) }
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
        Self(self.0.saturating_pow(exp))
    }

    #[must_use = "this returns the result of the operation, without modifying the original"]
    pub fn abs_diff(self, other: Self) -> Self {
        Self(self.0.abs_diff(other.0))
    }

    pub const fn parse_str_radix(src: &str, radix: u32) -> Self {
        Self(BaseU256::parse_str_radix(src, radix))
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

impl TryFrom<U256> for I256 {
    type Error = ConversionOverflowError;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        if value > I256::MAX.as_u256() {
            return Err(ConversionOverflowError::new("U256", "I256", value.to_string()))
        }

        Ok(value.as_i256())
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

        match BaseU256::from_str_radix(s, 10) {
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
        self.checked_shl(rhs)
            .expect("attempt to shift left with overflow")
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

impl ShlAssign<u32> for U256 {
    fn shl_assign(&mut self, rhs: u32) {
        *self = self.shl(rhs);
    }
}

impl<'a> ShlAssign<&'a u32> for U256 {
    fn shl_assign(&mut self, rhs: &'a u32) {
        *self = self.shl(*rhs);
    }
}

impl BitOrAssign for U256 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0.bitor_assign(rhs.0)
    }
}

impl<'a> BitOrAssign<&'a U256> for U256 {
    fn bitor_assign(&mut self, rhs: &'a U256) {
        self.0.bitor_assign(rhs.0)
    }
}

impl BitAndAssign for U256 {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0)
    }
}

impl<'a> BitAndAssign<&'a U256> for U256 {
    fn bitand_assign(&mut self, rhs: &'a U256) {
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

impl AsU256 for bool {
    fn as_u256(self) -> U256 {
        match self {
            true => U256::one(),
            false => U256::zero()
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{to_vec, from_slice};

    use super::*;

    #[test]
    fn size_of_works() {
        assert_eq!(std::mem::size_of::<U256>(), 32);
    }

    #[test]
    fn u256_new_works() {
        let num = U256::new([1; 32]);
        let a: [u8; 32] = num.to_be_bytes();
        assert_eq!(a, [1; 32]);

        let be_bytes = [
            0u8, 222u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 2u8, 3u8,
        ];
        let num = U256::new(be_bytes);
        let resulting_bytes: [u8; 32] = num.to_be_bytes();
        assert_eq!(be_bytes, resulting_bytes);
    }

    #[test]
    fn u256_zero_works() {
        let zero = U256::zero();
        assert_eq!(
            zero.to_be_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn uin256_one_works() {
        let one = U256::one();
        assert_eq!(
            one.to_be_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]
        );
    }

    #[test]
    fn u256_from_be_bytes() {
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(0u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 42,
        ]);
        assert_eq!(a, U256::from(42u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ]);
        assert_eq!(a, U256::from(1u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 0,
        ]);
        assert_eq!(a, U256::from(256u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 0, 0,
        ]);
        assert_eq!(a, U256::from(65536u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(16777216u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(4294967296u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1099511627776u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(281474976710656u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(72057594037927936u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(18446744073709551616u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(4722366482869645213696u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1208925819614629174706176u128));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1329227995784915872903807060280344576u128));

        // Values > u128::MAX
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 16));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 17));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 18));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 19));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 20));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 21));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 22));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 23));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 24));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 25));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 26));
        let a = U256::from_be_bytes([
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 27));
        let a = U256::from_be_bytes([
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 28));
        let a = U256::from_be_bytes([
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 29));
        let a = U256::from_be_bytes([
            0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 30));
        let a = U256::from_be_bytes([
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 31));
    }

    #[test]
    fn u256_from_le_bytes() {
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(0u128));
        let a = U256::from_le_bytes([
            42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(42u128));
        let a = U256::from_le_bytes([
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128));
        let a = U256::from_le_bytes([
            0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(256u128));
        let a = U256::from_le_bytes([
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(65536u128));
        let a = U256::from_le_bytes([
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(16777216u128));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(4294967296u128));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(72057594037927936u128));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(18446744073709551616u128));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1329227995784915872903807060280344576u128));

        // Values > u128::MAX
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 16));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 17));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 18));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 19));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 20));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 21));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 22));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 23));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 24));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 25));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 26));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 27));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 28));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 0, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 29));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 0,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 30));
        let a = U256::from_le_bytes([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ]);
        assert_eq!(a, U256::from(1u128) << (8 * 31));
    }

    #[test]
    fn u256_endianness() {
        let be_bytes = [
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 1u8, 2u8, 3u8,
        ];
        let le_bytes = [
            3u8, 2u8, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ];

        // These should all be the same.
        let num1 = U256::new(be_bytes);
        let num2 = U256::from_be_bytes(be_bytes);
        let num3 = U256::from_le_bytes(le_bytes);
        assert_eq!(num1, U256::from(65536u32 + 512 + 3));
        assert_eq!(num1, num2);
        assert_eq!(num1, num3);
    }

    #[test]
    fn u256_convert_from() {
        let a = U256::from(5u128);
        assert_eq!(a.0, BaseU256::from(5u32));

        let a = U256::from(5u64);
        assert_eq!(a.0, BaseU256::from(5u32));

        let a = U256::from(5u32);
        assert_eq!(a.0, BaseU256::from(5u32));

        let a = U256::from(5u16);
        assert_eq!(a.0, BaseU256::from(5u32));

        let a = U256::from(5u8);
        assert_eq!(a.0, BaseU256::from(5u32));

        let result = U256::try_from("34567");
        assert_eq!(
            result.unwrap().0,
            BaseU256::from_str_radix("34567", 10).unwrap()
        );

        let result = U256::try_from("1.23");
        assert!(result.is_err());
    }

    #[test]
    fn u256_convert_to_uint128() {
        let source = U256::from(42u128);
        let target = Uint128::try_from(source);
        assert_eq!(target, Ok(Uint128::new(42u128)));

        let source = U256::MAX;
        let target = Uint128::try_from(source);
        assert_eq!(
            target,
            Err(ConversionOverflowError::new(
                "U256",
                "Uint128",
                U256::MAX.to_string()
            ))
        );
    }

    #[test]
    fn u256_from_u128() {
        assert_eq!(
            U256::from_u128(123u128),
            U256::from_str("123").unwrap()
        );

        assert_eq!(
            U256::from_u128(9785746283745u128),
            U256::from_str("9785746283745").unwrap()
        );
    }

    #[test]
    fn u256_from_uint128() {
        assert_eq!(
            U256::from_uint128(Uint128::new(123)),
            U256::from_str("123").unwrap()
        );

        assert_eq!(
            U256::from_uint128(Uint128::new(9785746283745)),
            U256::from_str("9785746283745").unwrap()
        );
    }

    #[test]
    fn u256_implements_display() {
        let a = U256::from(12345u32);
        assert_eq!(format!("Embedded: {}", a), "Embedded: 12345");
        assert_eq!(a.to_string(), "12345");

        let a = U256::zero();
        assert_eq!(format!("Embedded: {}", a), "Embedded: 0");
        assert_eq!(a.to_string(), "0");
    }

    #[test]
    fn u256_display_padding_works() {
        let a = U256::from(123u64);
        assert_eq!(format!("Embedded: {:05}", a), "Embedded: 00123");
    }

    #[test]
    fn u256_to_be_bytes_works() {
        assert_eq!(
            U256::zero().to_be_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ]
        );
        assert_eq!(
            U256::MAX.to_be_bytes(),
            [
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff,
            ]
        );
        assert_eq!(
            U256::from(1u128).to_be_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1
            ]
        );
        // Python: `[b for b in (240282366920938463463374607431768124608).to_bytes(32, "big")]`
        assert_eq!(
            U256::from(240282366920938463463374607431768124608u128).to_be_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 180, 196, 179, 87, 165, 121, 59,
                133, 246, 117, 221, 191, 255, 254, 172, 192
            ]
        );
        assert_eq!(
            U256::from_be_bytes([
                233, 2, 240, 200, 115, 150, 240, 218, 88, 106, 45, 208, 134, 238, 119, 85, 22, 14,
                88, 166, 195, 154, 73, 64, 10, 44, 252, 96, 230, 187, 38, 29
            ])
            .to_be_bytes(),
            [
                233, 2, 240, 200, 115, 150, 240, 218, 88, 106, 45, 208, 134, 238, 119, 85, 22, 14,
                88, 166, 195, 154, 73, 64, 10, 44, 252, 96, 230, 187, 38, 29
            ]
        );
    }

    #[test]
    fn u256_to_le_bytes_works() {
        assert_eq!(
            U256::zero().to_le_bytes(),
            [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );
        assert_eq!(
            U256::MAX.to_le_bytes(),
            [
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff
            ]
        );
        assert_eq!(
            U256::from(1u128).to_le_bytes(),
            [
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        );
        // Python: `[b for b in (240282366920938463463374607431768124608).to_bytes(32, "little")]`
        assert_eq!(
            U256::from(240282366920938463463374607431768124608u128).to_le_bytes(),
            [
                192, 172, 254, 255, 191, 221, 117, 246, 133, 59, 121, 165, 87, 179, 196, 180, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        assert_eq!(
            U256::from_be_bytes([
                233, 2, 240, 200, 115, 150, 240, 218, 88, 106, 45, 208, 134, 238, 119, 85, 22, 14,
                88, 166, 195, 154, 73, 64, 10, 44, 252, 96, 230, 187, 38, 29
            ])
            .to_le_bytes(),
            [
                29, 38, 187, 230, 96, 252, 44, 10, 64, 73, 154, 195, 166, 88, 14, 22, 85, 119, 238,
                134, 208, 45, 106, 88, 218, 240, 150, 115, 200, 240, 2, 233
            ]
        );
    }

    #[test]
    fn u256_is_zero_works() {
        assert!(U256::zero().is_zero());
        assert!(U256(BaseU256::from(0u32)).is_zero());

        assert!(!U256::from(1u32).is_zero());
        assert!(!U256::from(123u32).is_zero());
    }

    #[test]
    fn u256_wrapping_methods() {
        // wrapping_add
        assert_eq!(
            U256::from(2u32).wrapping_add(U256::from(2u32)),
            U256::from(4u32)
        ); // non-wrapping
        assert_eq!(
            U256::MAX.wrapping_add(U256::from(1u32)),
            U256::from(0u32)
        ); // wrapping

        // wrapping_sub
        assert_eq!(
            U256::from(7u32).wrapping_sub(U256::from(5u32)),
            U256::from(2u32)
        ); // non-wrapping
        assert_eq!(
            U256::from(0u32).wrapping_sub(U256::from(1u32)),
            U256::MAX
        ); // wrapping

        // wrapping_mul
        assert_eq!(
            U256::from(3u32).wrapping_mul(U256::from(2u32)),
            U256::from(6u32)
        ); // non-wrapping
        assert_eq!(
            U256::MAX.wrapping_mul(U256::from(2u32)),
            U256::MAX - U256::one()
        ); // wrapping

        // wrapping_pow
        assert_eq!(U256::from(2u32).wrapping_pow(3), U256::from(8u32)); // non-wrapping
        assert_eq!(U256::MAX.wrapping_pow(2), U256::from(1u32)); // wrapping
    }

    #[test]
    fn u256_json() {
        let orig = U256::from(1234567890987654321u128);
        let serialized = to_vec(&orig).unwrap();
        assert_eq!(serialized.as_slice(), b"\"1234567890987654321\"");
        let parsed: U256 = from_slice(&serialized).unwrap();
        assert_eq!(parsed, orig);
    }

    #[test]
    fn u256_compare() {
        let a = U256::from(12345u32);
        let b = U256::from(23456u32);

        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, U256::from(12345u32));
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn u256_math() {
        let a = U256::from(12345u32);
        let b = U256::from(23456u32);

        // test + with owned and reference right hand side
        assert_eq!(a + b, U256::from(35801u32));
        assert_eq!(a + &b, U256::from(35801u32));

        // test - with owned and reference right hand side
        assert_eq!(b - a, U256::from(11111u32));
        assert_eq!(b - &a, U256::from(11111u32));

        // test += with owned and reference right hand side
        let mut c = U256::from(300000u32);
        c += b;
        assert_eq!(c, U256::from(323456u32));
        let mut d = U256::from(300000u32);
        d += &b;
        assert_eq!(d, U256::from(323456u32));

        // test -= with owned and reference right hand side
        let mut c = U256::from(300000u32);
        c -= b;
        assert_eq!(c, U256::from(276544u32));
        let mut d = U256::from(300000u32);
        d -= &b;
        assert_eq!(d, U256::from(276544u32));

        // error result on underflow (- would produce negative result)
        let underflow_result = a.checked_sub(b);
        let OverflowError {
            operand1, operand2, ..
        } = underflow_result.unwrap_err();
        assert_eq!((operand1, operand2), (a.to_string(), b.to_string()));
    }

    #[test]
    #[should_panic]
    fn u256_add_overflow_panics() {
        let max = U256::new([255u8; 32]);
        let _ = max + U256::from(12u32);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn u256_sub_works() {
        assert_eq!(
            U256::from(2u32) - U256::from(1u32),
            U256::from(1u32)
        );
        assert_eq!(
            U256::from(2u32) - U256::from(0u32),
            U256::from(2u32)
        );
        assert_eq!(
            U256::from(2u32) - U256::from(2u32),
            U256::from(0u32)
        );

        // works for refs
        let a = U256::from(10u32);
        let b = U256::from(3u32);
        let expected = U256::from(7u32);
        assert_eq!(a - b, expected);
        assert_eq!(a - &b, expected);
        assert_eq!(&a - b, expected);
        assert_eq!(&a - &b, expected);
    }

    #[test]
    #[should_panic]
    fn u256_sub_overflow_panics() {
        let _ = U256::from(1u32) - U256::from(2u32);
    }

    #[test]
    fn u256_sub_assign_works() {
        let mut a = U256::from(14u32);
        a -= U256::from(2u32);
        assert_eq!(a, U256::from(12u32));

        // works for refs
        let mut a = U256::from(10u32);
        let b = U256::from(3u32);
        let expected = U256::from(7u32);
        a -= &b;
        assert_eq!(a, expected);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn u256_mul_works() {
        assert_eq!(
            U256::from(2u32) * U256::from(3u32),
            U256::from(6u32)
        );
        assert_eq!(U256::from(2u32) * U256::zero(), U256::zero());

        // works for refs
        let a = U256::from(11u32);
        let b = U256::from(3u32);
        let expected = U256::from(33u32);
        assert_eq!(a * b, expected);
        assert_eq!(a * &b, expected);
        assert_eq!(&a * b, expected);
        assert_eq!(&a * &b, expected);
    }

    #[test]
    fn u256_mul_assign_works() {
        let mut a = U256::from(14u32);
        a *= U256::from(2u32);
        assert_eq!(a, U256::from(28u32));

        // works for refs
        let mut a = U256::from(10u32);
        let b = U256::from(3u32);
        a *= &b;
        assert_eq!(a, U256::from(30u32));
    }

    #[test]
    fn u256_pow_works() {
        assert_eq!(U256::from(2u32).pow(2), U256::from(4u32));
        assert_eq!(U256::from(2u32).pow(10), U256::from(1024u32));
    }

    #[test]
    #[should_panic]
    fn u256_pow_overflow_panics() {
        _ = U256::MAX.pow(2u32);
    }

    #[test]
    fn u256_shr_works() {
        let original = U256::new([
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 2u8, 0u8, 4u8, 2u8,
        ]);

        let shifted = U256::new([
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 128u8, 1u8, 0u8,
        ]);

        assert_eq!(original >> 2u32, shifted);
    }

    #[test]
    #[should_panic]
    fn u256_shr_overflow_panics() {
        let _ = U256::from(1u32) >> 256u32;
    }

    #[test]
    fn u256_shl_works() {
        let original = U256::new([
            64u8, 128u8, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ]);

        let shifted = U256::new([
            2u8, 0u8, 4u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
        ]);

        assert_eq!(original << 2u32, shifted);
    }

    #[test]
    #[should_panic]
    fn u256_shl_overflow_panics() {
        let _ = U256::from(1u32) << 256u32;
    }

    #[test]
    fn sum_works() {
        let nums = vec![
            U256::from(17u32),
            U256::from(123u32),
            U256::from(540u32),
            U256::from(82u32),
        ];
        let expected = U256::from(762u32);

        let sum_as_ref: U256 = nums.iter().sum();
        assert_eq!(expected, sum_as_ref);

        let sum_as_owned: U256 = nums.into_iter().sum();
        assert_eq!(expected, sum_as_owned);
    }

    #[test]
    fn u256_methods() {
        // checked_*
        assert!(matches!(
            U256::MAX.checked_add(U256::from(1u32)),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            U256::from(1u32).checked_add(U256::from(1u32)),
            Ok(U256::from(2u32)),
        );
        assert!(matches!(
            U256::from(0u32).checked_sub(U256::from(1u32)),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            U256::from(2u32).checked_sub(U256::from(1u32)),
            Ok(U256::from(1u32)),
        );
        assert!(matches!(
            U256::MAX.checked_mul(U256::from(2u32)),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            U256::from(2u32).checked_mul(U256::from(2u32)),
            Ok(U256::from(4u32)),
        );
        assert!(matches!(
            U256::MAX.checked_pow(2u32),
            Err(OverflowError { .. })
        ));
        assert_eq!(
            U256::from(2u32).checked_pow(3u32),
            Ok(U256::from(8u32)),
        );
        assert!(matches!(
            U256::MAX.checked_div(U256::from(0u32)),
            Err(DivideByZeroError { .. })
        ));
        assert_eq!(
            U256::from(6u32).checked_div(U256::from(2u32)),
            Ok(U256::from(3u32)),
        );
        assert!(matches!(
            U256::MAX.checked_div_euclid(U256::from(0u32)),
            Err(DivideByZeroError { .. })
        ));
        assert_eq!(
            U256::from(6u32).checked_div_euclid(U256::from(2u32)),
            Ok(U256::from(3u32)),
        );
        assert_eq!(
            U256::from(7u32).checked_div_euclid(U256::from(2u32)),
            Ok(U256::from(3u32)),
        );
        assert!(matches!(
            U256::MAX.checked_rem(U256::from(0u32)),
            Err(DivideByZeroError { .. })
        ));

        // saturating_*
        assert_eq!(
            U256::MAX.saturating_add(U256::from(1u32)),
            U256::MAX
        );
        assert_eq!(
            U256::from(0u32).saturating_sub(U256::from(1u32)),
            U256::from(0u32)
        );
        assert_eq!(
            U256::MAX.saturating_mul(U256::from(2u32)),
            U256::MAX
        );
        assert_eq!(
            U256::from(4u32).saturating_pow(2u32),
            U256::from(16u32)
        );
        assert_eq!(U256::MAX.saturating_pow(2u32), U256::MAX);
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn u256_implements_rem() {
        let a = U256::from(10u32);
        assert_eq!(a % U256::from(10u32), U256::zero());
        assert_eq!(a % U256::from(2u32), U256::zero());
        assert_eq!(a % U256::from(1u32), U256::zero());
        assert_eq!(a % U256::from(3u32), U256::from(1u32));
        assert_eq!(a % U256::from(4u32), U256::from(2u32));

        // works for refs
        let a = U256::from(10u32);
        let b = U256::from(3u32);
        let expected = U256::from(1u32);
        assert_eq!(a % b, expected);
        assert_eq!(a % &b, expected);
        assert_eq!(&a % b, expected);
        assert_eq!(&a % &b, expected);
    }

    #[test]
    #[should_panic(expected = "divisor of zero")]
    fn u256_rem_panics_for_zero() {
        let _ = U256::from(10u32) % U256::zero();
    }

    #[test]
    #[allow(clippy::op_ref)]
    fn u256_rem_works() {
        assert_eq!(
            U256::from(12u32) % U256::from(10u32),
            U256::from(2u32)
        );
        assert_eq!(U256::from(50u32) % U256::from(5u32), U256::zero());

        // works for refs
        let a = U256::from(42u32);
        let b = U256::from(5u32);
        let expected = U256::from(2u32);
        assert_eq!(a % b, expected);
        assert_eq!(a % &b, expected);
        assert_eq!(&a % b, expected);
        assert_eq!(&a % &b, expected);
    }

    #[test]
    fn u256_rem_assign_works() {
        let mut a = U256::from(30u32);
        a %= U256::from(4u32);
        assert_eq!(a, U256::from(2u32));

        // works for refs
        let mut a = U256::from(25u32);
        let b = U256::from(6u32);
        a %= &b;
        assert_eq!(a, U256::from(1u32));
    }

    #[test]
    fn u256_abs_diff_works() {
        let a = U256::from(42u32);
        let b = U256::from(5u32);
        let expected = U256::from(37u32);
        assert_eq!(a.abs_diff(b), expected);
        assert_eq!(b.abs_diff(a), expected);
    }

    #[test]
    fn u256_partial_eq() {
        let test_cases = [(1, 1, true), (42, 42, true), (42, 24, false), (0, 0, true)]
            .into_iter()
            .map(|(lhs, rhs, expected): (u64, u64, bool)| {
                (U256::from(lhs), U256::from(rhs), expected)
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
