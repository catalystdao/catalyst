
// ******************************************************************************************************************************
// Catalyst payload structure
// ******************************************************************************************************************************

// Common Payload               Start       Length
//    CONTEXT                   0           1
//    + FROM_VAULT              1           65
//    + TO_VAULT                66          65
//    + TO_ACCOUNT              131         65
//    + UNITS                   196         32
// 
// Context-depending Payload
//    CTX0 - 0x00 - Asset Swap Payload
//       + TO_ASSET_INDEX       228         1
//       + MIN_OUT              229         32
//       + FROM_AMOUNT          261         32
//       + FROM_ASSET           293         65
//       + BLOCK_NUMBER         358         4
//       + UW_INCENTIVE         362         2       (underwrite incentive)
//
//    CTX1 - 0x01 - Liquidity Swap Payload
//       + MIN_VAULT_TOKENS     228         32
//       + MIN_REFERENCE        260         32
//       + FROM_AMOUNT          292         32
//       + BLOCK_NUMBER         324         4
//
// Calldata
//    + DATA_LENGTH             LENGTH-N-2  2
//    + DATA                    LENGTH-N    N


// Contexts *********************************************************************************************************************

pub const CTX0_ASSET_SWAP            : u8 = 0x00;
pub const CTX1_LIQUIDITY_SWAP        : u8 = 0x01;



// Common Payload ***************************************************************************************************************

pub const CONTEXT_POS                : usize = 0;

pub const FROM_VAULT_START           : usize = 1;
pub const FROM_VAULT_END             : usize = 66;

pub const TO_VAULT_START             : usize = 66;
pub const TO_VAULT_END               : usize = 131;

pub const TO_ACCOUNT_START           : usize = 131;
pub const TO_ACCOUNT_END             : usize = 196;

pub const UNITS_START                : usize = 196;
pub const UNITS_END                  : usize = 228;



// CTX0 Asset Swap Payload ******************************************************************************************************

pub const CTX0_TO_ASSET_INDEX_POS    : usize = 228;

pub const CTX0_MIN_OUT_START         : usize = 229;
pub const CTX0_MIN_OUT_END           : usize = 261;

pub const CTX0_FROM_AMOUNT_START     : usize = 261;
pub const CTX0_FROM_AMOUNT_END       : usize = 293;

pub const CTX0_FROM_ASSET_START      : usize = 293;
pub const CTX0_FROM_ASSET_END        : usize = 358;

pub const CTX0_BLOCK_NUMBER_START    : usize = 358;
pub const CTX0_BLOCK_NUMBER_END      : usize = 362;

pub const CTX0_UW_INCENTIVE_START    : usize = 362;
pub const CTX0_UW_INCENTIVE_END      : usize = 364;

pub const CTX0_DATA_LENGTH_START     : usize = 364;
pub const CTX0_DATA_LENGTH_END       : usize = 366;

pub const CTX0_DATA_START            : usize = 366;



// CTX1 Liquidity Swap Payload **************************************************************************************************

pub const CTX1_MIN_VAULT_TOKEN_START : usize = 228;
pub const CTX1_MIN_VAULT_TOKEN_END   : usize = 260;

pub const CTX1_MIN_REFERENCE_START   : usize = 260;
pub const CTX1_MIN_REFERENCE_END     : usize = 292;

pub const CTX1_FROM_AMOUNT_START     : usize = 292;
pub const CTX1_FROM_AMOUNT_END       : usize = 324;

pub const CTX1_BLOCK_NUMBER_START    : usize = 324;
pub const CTX1_BLOCK_NUMBER_END      : usize = 328;

pub const CTX1_DATA_LENGTH_START     : usize = 328;
pub const CTX1_DATA_LENGTH_END       : usize = 330;

pub const CTX1_DATA_START            : usize = 330;



// CosmWasm Calldata Specific Payload *******************************************************************************************

pub const CALLDATA_TARGET_START      : usize = 0;
pub const CALLDATA_TARGET_END        : usize = 65;

pub const CALLDATA_BYTES_START       : usize = 65;





// ******************************************************************************************************************************
// Payload Helpers
// ******************************************************************************************************************************
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Deps, Addr, Binary};
use catalyst_types::U256;
use crate::ContractError;


// Catalyst Packet **************************************************************************************************************

pub type CatalystV1SendAssetPayload = CatalystV1Payload<SendAssetVariablePayload>;
pub type CatalystV1SendLiquidityPayload = CatalystV1Payload<SendLiquidityVariablePayload>;

// The CatalystV1Packet enum describes the different structures the Catalyst payload may take
pub enum CatalystV1Packet {
    SendAsset(CatalystV1SendAssetPayload),
    SendLiquidity(CatalystV1SendLiquidityPayload)
}

impl CatalystV1Packet {

    #[cfg(test)]
    pub fn try_encode(
        &self
    ) -> Result<Binary, ContractError> {
        
        match self {
            CatalystV1Packet::SendAsset(payload) => payload.try_encode(),
            CatalystV1Packet::SendLiquidity(payload) => payload.try_encode(),
        }

    }

    pub fn try_decode(
        data: Binary
    ) -> Result<CatalystV1Packet, ContractError> {

        let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;

        match *context {
            CTX0_ASSET_SWAP => Ok(CatalystV1Packet::SendAsset(
                CatalystV1SendAssetPayload::try_decode(data)?
            )),
            CTX1_LIQUIDITY_SWAP => Ok(CatalystV1Packet::SendLiquidity(
                CatalystV1SendLiquidityPayload::try_decode(data)?
            )),
            _ => return Err(ContractError::PayloadDecodingError {})
        }

    }

}



// Catalyst Payload *************************************************************************************************************

/// CatalystV1Payload is used to encode/decode a Catalyst payload. It itself encodes/decodes 
/// the common part of the Catalyst payload, and uses a generic `variable_payload` to 
/// encode/decode the variable part of the payload.
/// 
/// # Fields:
/// * `from_vault` - The source vault.
/// * `to_vault` - The target vault.
/// * `to_account` - The destination account.
/// * `u` - The transferred units.
/// * `variable_payload` - Data specific to one of the possible Catalyst payload configurations.
/// 
pub struct CatalystV1Payload<T: CatalystV1VariablePayload> {
    pub from_vault: CatalystEncodedAddress,
    pub to_vault: CatalystEncodedAddress,
    pub to_account: CatalystEncodedAddress,
    pub u: U256,
    pub variable_payload: T
}

/// Trait to be implemented by the extensions of the base Catalyst payload struct.
pub trait CatalystV1VariablePayload: Sized {
    const CONTEXT: u8;
    fn size(&self) -> usize;
    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError>;
    fn try_decode(buffer: Vec<u8>) -> Result<Self, ContractError>;
}

impl<T: CatalystV1VariablePayload> CatalystV1Payload<T> {


    /// Get the size of the payload in bytes.
    pub fn size(&self) -> usize {
        UNITS_END - CONTEXT_POS
            + self.variable_payload.size()
    }


    /// Encode the payload into a binary representation.
    pub fn try_encode(
        &self
    ) -> Result<Binary, ContractError> {

        // Preallocate the required size for the payload to avoid runtime reallocations.
        let mut data: Vec<u8> = Vec::with_capacity(self.size());   
    
        // Encode the common part of the payload
        data.push(T::CONTEXT);
        data.extend_from_slice(self.from_vault.as_ref());
        data.extend_from_slice(self.to_vault.as_ref());
        data.extend_from_slice(self.to_account.as_ref());
        data.extend_from_slice(&self.u.to_be_bytes());

        // Encode the variable part of the payload
        self.variable_payload.try_encode(&mut data)?;
    
        Ok(Binary(data))

    }


    /// Decode a vector of bytes into a Catalyst payload.
    pub fn try_decode(
        data: Binary
    ) -> Result<CatalystV1Payload<T>, ContractError> {

        // Make sure the 'context' of the payload matches the expected one.
        let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;
        if context != &T::CONTEXT {
            return Err(ContractError::PayloadDecodingError {});
        }
    
        // Decode the common part of the payload
        // NOTE: The decoded address are not verified at this point, as they might not get used by the
        // implementation during the lifetime of the struct.
        let from_vault = CatalystEncodedAddress::from_slice_unchecked(
            data.get(FROM_VAULT_START .. FROM_VAULT_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );

        let to_vault = CatalystEncodedAddress::from_slice_unchecked(
            data.get(TO_VAULT_START .. TO_VAULT_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );

        let to_account = CatalystEncodedAddress::from_slice_unchecked(
            data.get(TO_ACCOUNT_START .. TO_ACCOUNT_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );

        let u = U256::from_be_bytes(
            data.get(UNITS_START .. UNITS_END)
                .ok_or(ContractError::PayloadDecodingError {})?
                .try_into().unwrap()    // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart this should never panic.
        );

        // Decode the variable part of the payload
        let variable_payload = T::try_decode(data.0)?;


        return Ok(
            Self {
                from_vault,
                to_vault,
                to_account,
                u,
                variable_payload
            }
        )

    }

}



// Payload Implementations ******************************************************************************************************

/// Type for decoded calldata.
#[cw_serde]
pub struct CatalystCalldata {
    pub target: Addr,
    pub bytes: Binary
}


/// Send asset variable payload.
/// 
/// # Fields:
/// * `to_asset_index` - The target asset index.
/// * `min_out` - The minimum output.
/// * `from_amount` - The source asset amount.
/// * `from_asset` - The source asset.
/// * `block_number` - The block number at which the transaction was committed.
/// * `underwrite_incentive_x16` - The underwrite incentive.
/// * `calldata` - Arbitrary data to be executed on the destination.
/// 
pub struct SendAssetVariablePayload {
    pub to_asset_index: u8,
    pub min_out: U256,
    pub from_amount: U256,
    pub from_asset: CatalystEncodedAddress,
    pub block_number: u32,
    pub underwrite_incentive_x16: u16,
    pub calldata: Binary
}

impl CatalystV1VariablePayload for SendAssetVariablePayload {


    /// The context of the payload.
    const CONTEXT: u8 = CTX0_ASSET_SWAP;


    /// Get the size of the payload
    fn size(&self) -> usize {
        CTX0_DATA_START - CTX0_TO_ASSET_INDEX_POS
            + self.calldata.len()
    }


    /// Encode the variable payload into a binary representation.
    /// 
    /// # Arguments:
    /// * `buffer` - Buffer into which to encode the payload.
    /// 
    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError> {

        buffer.push(self.to_asset_index);
        buffer.extend_from_slice(&self.min_out.to_be_bytes());
        buffer.extend_from_slice(&self.from_amount.to_be_bytes());
        buffer.extend_from_slice(self.from_asset.as_ref());
        buffer.extend_from_slice(&self.block_number.to_be_bytes());
        buffer.extend_from_slice(&self.underwrite_incentive_x16.to_be_bytes());
    
        let calldata_length: u16 = self.calldata.len().try_into()
            .map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow.
        buffer.extend_from_slice(&calldata_length.to_be_bytes());
        buffer.extend_from_slice(&self.calldata);

        Ok(())
    }


    /// Decode a vector of bytes into a 'SendAsset' payload (variable part).
    /// 
    /// # Arguments:
    /// * `buffer` - They bytes to be decoded.
    /// 
    fn try_decode(buffer: Vec<u8>) -> Result<Self, ContractError> {

        let to_asset_index = buffer.get(CTX0_TO_ASSET_INDEX_POS)
            .ok_or(ContractError::PayloadDecodingError {})?.clone();

        let min_out = U256::from_be_bytes(
            buffer.get(
                CTX0_MIN_OUT_START .. CTX0_MIN_OUT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX0_MIN_OUT_START' and 'CTX0_MIN_OUT_END' are 32 bytes apart this should never panic.
        );

        let from_amount = U256::from_be_bytes(
            buffer.get(
                CTX0_FROM_AMOUNT_START .. CTX0_FROM_AMOUNT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX0_FROM_AMOUNT_START' and 'CTX0_FROM_AMOUNT_END' are 32 bytes apart this should never panic.
        );

        // NOTE: The decoded address is not verified at this point, as it might not get used by the
        // implementation during the lifetime of the struct.
        let from_asset = CatalystEncodedAddress::from_slice_unchecked(
            buffer.get(CTX0_FROM_ASSET_START .. CTX0_FROM_ASSET_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );

        let block_number = u32::from_be_bytes(
            buffer.get(
                CTX0_BLOCK_NUMBER_START .. CTX0_BLOCK_NUMBER_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX0_BLOCK_NUMBER_START' and 'CTX0_BLOCK_NUMBER_END' are 4 bytes apart this should never panic.
        );

        let underwrite_incentive_x16 = u16::from_be_bytes(
            buffer.get(
                CTX0_UW_INCENTIVE_START .. CTX0_UW_INCENTIVE_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX0_UW_INCENTIVE_START' and 'CTX0_UW_INCENTIVE_END' are 2 bytes apart this should never panic.
        );

        let calldata_length: usize = u16::from_be_bytes(
            buffer.get(
                CTX0_DATA_LENGTH_START .. CTX0_DATA_LENGTH_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX0_DATA_LENGTH_START' and 'CTX0_DATA_LENGTH_END' are 2 bytes apart this should never panic.
        ) as usize;

        let calldata = Binary(
            buffer.get(
                CTX0_DATA_START .. CTX0_DATA_START + calldata_length
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
        );

        Ok(SendAssetVariablePayload {
            to_asset_index,
            min_out,
            from_amount,
            from_asset,
            block_number,
            underwrite_incentive_x16,
            calldata
        })

    }

}


/// Send liquidity variable payload.
/// 
/// # Fields:
/// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
/// * `min_reference_asset` - The mininum reference asset value on the target vault.
/// * `from_amount` - The source vault tokens amount.
/// * `block_number` - The block number at which the transaction was committed.
/// * `calldata` - Arbitrary data to be executed on the destination.
/// 
pub struct SendLiquidityVariablePayload {
    pub min_vault_tokens: U256,
    pub min_reference_asset: U256,
    pub from_amount: U256,
    pub block_number: u32,
    pub calldata: Binary
}

impl CatalystV1VariablePayload for SendLiquidityVariablePayload {


    /// The context of the payload.
    const CONTEXT: u8 = CTX1_LIQUIDITY_SWAP;


    /// Get the size of the payload
    fn size(&self) -> usize {
        CTX1_DATA_START - CTX1_MIN_VAULT_TOKEN_START
            + self.calldata.len()
    }


    /// Encode the variable payload into a binary representation.
    /// 
    /// # Arguments:
    /// * `buffer` - Buffer into which to encode the payload.
    /// 
    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError> {
    
        buffer.extend_from_slice(&self.min_vault_tokens.to_be_bytes());
        buffer.extend_from_slice(&self.min_reference_asset.to_be_bytes());
        buffer.extend_from_slice(&self.from_amount.to_be_bytes());
        buffer.extend_from_slice(&self.block_number.to_be_bytes());

        let calldata_length: u16 = self.calldata.len().try_into()
            .map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow.
        buffer.extend_from_slice(&calldata_length.to_be_bytes());
        buffer.extend_from_slice(&self.calldata);

        Ok(())
    }


    /// Decode a vector of bytes into a 'SendLiquidity' payload (variable part).
    /// 
    /// # Arguments:
    /// * `buffer` - They bytes to be decoded.
    /// 
    fn try_decode(buffer: Vec<u8>) -> Result<Self, ContractError> {

        let min_vault_tokens = U256::from_be_bytes(
            buffer.get(
                CTX1_MIN_VAULT_TOKEN_START .. CTX1_MIN_VAULT_TOKEN_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX1_MIN_VAULT_TOKEN_START' and 'CTX1_MIN_VAULT_TOKEN_END' are 32 bytes apart this should never panic.
        );

        let min_reference_asset = U256::from_be_bytes(
            buffer.get(
                CTX1_MIN_REFERENCE_START .. CTX1_MIN_REFERENCE_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX1_MIN_REFERENCE_START' and 'CTX1_MIN_REFERENCE_END' are 32 bytes apart this should never panic.
        );

        let from_amount = U256::from_be_bytes(
            buffer.get(
                CTX1_FROM_AMOUNT_START .. CTX1_FROM_AMOUNT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX1_FROM_AMOUNT_START' and 'CTX1_FROM_AMOUNT_END' are 32 bytes apart this should never panic.
        );

        let block_number = u32::from_be_bytes(
            buffer.get(
                CTX1_BLOCK_NUMBER_START .. CTX1_BLOCK_NUMBER_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX1_BLOCK_NUMBER_START' and 'CTX1_BLOCK_NUMBER_END' are 4 bytes apart this should never panic.
        );

        let calldata_length: usize = u16::from_be_bytes(
            buffer.get(
                CTX1_DATA_LENGTH_START .. CTX1_DATA_LENGTH_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()      // If 'CTX1_DATA_LENGTH_START' and 'CTX1_DATA_LENGTH_END' are 2 bytes apart this should never panic.
        ) as usize;

        let calldata = Binary(
            buffer.get(
                CTX1_DATA_START ..
                CTX1_DATA_START + calldata_length
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
        );

        Ok(SendLiquidityVariablePayload {
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            block_number,
            calldata
        })

    }
}




// Misc helpers *****************************************************************************************************************

/// Parse calldata bytes. Returns *None* for no calldata.
/// 
/// **NOTE**: Unlike with all the other payload helpers, the `target` address **does** get validated. This
/// is to avoid executing the entire swap logic should an invalid address be provided. Note that this step
/// is not critical for the safety of the protocol.
/// 
/// # Arguments:
/// * `calldata` - Bytes to parse.
/// 
pub fn parse_calldata(
    deps: Deps,
    calldata: Binary
) -> Result<Option<CatalystCalldata>, ContractError> {

    if calldata.len() == 0 {
        return Ok(None);
    }

    let target_bytes = CatalystEncodedAddress::from_slice_unchecked(
        calldata.get(CALLDATA_TARGET_START..CALLDATA_TARGET_END).ok_or(ContractError::PayloadDecodingError {})?
    ).try_decode()?;

    let target = deps.api.addr_validate(
        String::from_utf8(target_bytes).map_err(|_| ContractError::PayloadDecodingError {})?.as_str()
    )?;

    let bytes = Binary(
        calldata.get(CALLDATA_BYTES_START..).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
    );

    Ok(
        Some(CatalystCalldata {
            target,
            bytes
        })
    )
}


/// Wrapper around a bytes vec for encoding/decoding of Catalyst's 65-byte payload addresses.
pub struct CatalystEncodedAddress([u8; 65]);

impl AsRef<[u8]> for CatalystEncodedAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0.as_ref()
    }
}

impl Into<Vec<u8>> for CatalystEncodedAddress {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Into<Binary> for CatalystEncodedAddress {
    fn into(self) -> Binary {
        Binary(self.0.to_vec())
    }
}

impl TryFrom<Vec<u8>> for CatalystEncodedAddress {
    type Error = ContractError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {

        let bytes: [u8; Self::LENGTH] = value
            .try_into()
            .map_err(|_| ContractError::InvalidCatalystEncodedAddress {})?;

        if bytes[0] as usize >= Self::LENGTH {
            return Err(ContractError::InvalidCatalystEncodedAddress {})
        }

        Ok(Self(bytes))
    }
}

impl TryFrom<Binary> for CatalystEncodedAddress {
    type Error = ContractError;

    fn try_from(value: Binary) -> Result<Self, Self::Error> {

        let bytes: [u8; Self::LENGTH] = value.0
            .try_into()
            .map_err(|_| ContractError::InvalidCatalystEncodedAddress {})?;

        if bytes[0] as usize >= Self::LENGTH {
            return Err(ContractError::InvalidCatalystEncodedAddress {})
        }

        Ok(Self(bytes))
    }
}

impl TryFrom<&[u8]> for CatalystEncodedAddress {
    type Error = ContractError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {

        let bytes: [u8; Self::LENGTH] = value
            .try_into()
            .map_err(|_| ContractError::InvalidCatalystEncodedAddress {})?;

        if bytes[0] as usize >= Self::LENGTH {
            return Err(ContractError::InvalidCatalystEncodedAddress {})
        }

        Ok(Self(bytes))
    }
}


impl CatalystEncodedAddress {

    const LENGTH: usize = 65;

    /// Return a vector representation of the address.
    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Return a Binary representation of the address.
    pub fn to_binary(self) -> Binary {
        Binary(self.0.to_vec())
    }

    /// Create a CatalystEncodedAddress from a slice.
    /// 
    /// ! **IMPORTANT!**: This method must only be used when it is guaranteed that data.len() is 65 bytes long.
    /// 
    /// # Arguments:
    /// * `data` - The slice from which to create the CatalystEncodedAddress.
    /// 
    pub fn from_slice_unchecked(data: &[u8]) -> Self {
        Self(data.try_into().unwrap())
    }

    /// Try to encode an address from a slice. The encoded address always has fixed length
    /// (`LENGTH`), and is of the following form:
    ///     <Address length (1 byte)> <Zero padding> <Address>
    /// 
    /// # Arguments:
    /// * `address` - The slice that is to be encoded.
    /// 
    pub fn try_encode(address: &[u8]) -> Result<Self, ContractError> {

        let address_len = address.len();
        if address_len > (Self::LENGTH - 1) {   // Subtracting '1', as the first byte of the encoded 
                                                // address is reserved for the address length
            return Err(ContractError::PayloadEncodingError {});
        }

        let mut encoded_address = [0u8; Self::LENGTH];
        encoded_address[0] = address_len as u8;     // Casting to u8 is safe, as address_len is < Self::LENGTH < u8.max

        encoded_address[Self::LENGTH-address_len..].copy_from_slice(address.as_ref());

        Ok(Self(encoded_address))
    }

    /// Try to decode a Catalyst-encoded address into a vector of bytes.
    pub fn try_decode(&self) -> Result<Vec<u8>, ContractError> {

        let address_length: usize = *self.0.get(0)
            .ok_or(ContractError::PayloadDecodingError {})? as usize;

        // Get the last <address_length> bytes of the encoded address.
        let address_start_byte = Self::LENGTH
            .checked_sub(address_length)
            .ok_or(ContractError::InvalidCatalystEncodedAddress {})?;

        self.0.get(address_start_byte..)
            .ok_or(ContractError::PayloadDecodingError {})
            .map(|slice| slice.to_vec())

    }

    /// Try to decode a Catalyst-encoded address into a utf8 string.
    pub fn try_decode_as_string(&self) -> Result<String, ContractError> {

        String::from_utf8(
            self.try_decode()?
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }
}
