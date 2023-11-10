use core::fmt;
use cosmwasm_std::{StdResult, Binary, StdError};
use cw_storage_plus::PrimaryKey;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};


#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, JsonSchema, Serialize, Deserialize)]
pub struct Bytes32(pub [u8; 32]);

impl Bytes32 {

    /// Take an (untrusted) string and decode it into bytes.
    /// fails if it is not valid base64
    pub fn from_base64(encoded: &str) -> StdResult<Self> {

        let bytes: [u8; 32] = Binary::from_base64(encoded)?.0
            .try_into()
            .map_err(|bytes_vec: Vec<u8>| {
                StdError::InvalidDataSize { expected: 32, actual: bytes_vec.len() as u64 }
            })?;

        Ok(Self(bytes))
    }

    /// Encode to base64 string (guaranteed to be success as we control the data inside).
    /// this returns normalized form (with trailing = if needed)
    pub fn to_base64(&self) -> String {
        Binary::from(self.0).to_base64()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

}

impl fmt::Display for Bytes32 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Binary::from(self.0).fmt(f)
    }
}

impl fmt::Debug for Bytes32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Bytes32> for Vec<u8> {
    fn from(value: Bytes32) -> Vec<u8> {
        value.0.to_vec()
    }
}

impl TryFrom<Vec<u8>> for Bytes32 {
    type Error = StdError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {

        let bytes: [u8; 32] = value
            .try_into()
            .map_err(|bytes_vec: Vec<u8>| {
                StdError::InvalidDataSize { expected: 32, actual: bytes_vec.len() as u64 }
            })?;

        Ok(Self(bytes))
    }
}

impl<'a> PrimaryKey<'a> for Bytes32 {
    type Prefix = <[u8; 32] as PrimaryKey<'a>>::Prefix;

    type SubPrefix = <[u8; 32] as PrimaryKey<'a>>::SubPrefix;

    type Suffix = <[u8; 32] as PrimaryKey<'a>>::Suffix;

    type SuperSuffix = <[u8; 32] as PrimaryKey<'a>>::SuperSuffix;

    fn key(&self) -> Vec<cw_storage_plus::Key> {
        self.0.key()
    }
}
