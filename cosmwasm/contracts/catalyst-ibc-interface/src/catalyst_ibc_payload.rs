
// ******************************************************************************************************************************
// Catalyst IBC payload structure
// ******************************************************************************************************************************

// Common Payload (beginning)
//    CONTEXT                   0   (1 byte)
//    + FROM_VAULT              1   (65 bytes)
//    + TO_VAULT                66  (65 bytes)
//    + TO_ACCOUNT              131 (65 bytes)
//    + UNITS                   196 (32 bytes)
// 
// Context-depending Payload
//    CTX0 - 0x00 - Asset Swap Payload
//       + TO_ASSET_INDEX       228 (1 byte)
//       + MIN_OUT              229 (32 bytes)
//       + FROM_AMOUNT          261 (32 bytes)
//       + FROM_ASSET           293 (65 bytes)
//       + BLOCK_NUMBER         358 (4 bytes)
//
//    CTX1 - 0x01 - Liquidity Swap Payload
//       + MIN_OUT              228 (32 bytes)
//       + FROM_AMOUNT          260 (32 bytes)
//       + BLOCK_NUMBER         292 (4 bytes)
//
// Common Payload (end)
//    + DATA_LENGTH             LENGTH-N-2 (2 bytes)
//    + DATA                    LENGTH-N   (N bytes)


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

pub const CTX0_DATA_LENGTH_START     : usize = 362;
pub const CTX0_DATA_LENGTH_END       : usize = 364;

pub const CTX0_DATA_START            : usize = 364;



// CTX1 Liquidity Swap Payload **************************************************************************************************

pub const CTX1_MIN_POOL_TOKEN_START  : usize = 228;
pub const CTX1_MIN_POOL_TOKEN_END    : usize = 260;

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
use cosmwasm_std::{Deps, Addr, Uint128, Binary};
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

    //TODO pass the 'IbcPacket' to try_decode and return a copy of the data and not references to it?
    //TODO Or make CatalystV1SendAssetPayload/CatalystV1SendLiquidityPayload accept the IBC packet directly (rename them to CatalystV1SendAssetPACKET)
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



// Catalyst payload *************************************************************************************************************

// The CatalystV1Payload struct is used to encode/decode a Catalyst payload. It itself encodes/decodes the common part of the
// Catalyst payload, and uses a generic `variable_payload` to encode/decode the variable part of the payload.


pub struct CatalystV1Payload<T: CatalystV1VariablePayload> {
    pub from_vault: CatalystEncodedAddress,
    pub to_vault: CatalystEncodedAddress,
    pub to_account: CatalystEncodedAddress,
    pub u: U256,
    pub variable_payload: T
}

impl<T: CatalystV1VariablePayload> CatalystV1Payload<T> {

    pub fn size(&self) -> usize {
        // Addition is way below the overflow threshold
        1                                   // Context
        + 65                                // From vault
        + 65                                // To vault
        + 65                                // To account
        + 32                                // Units
        + self.variable_payload.size()
    }

    pub fn try_encode(
        &self
    ) -> Result<Binary, ContractError> {

        // Preallocate the required size for the IBC payload to avoid runtime reallocations.
        let mut data: Vec<u8> = Vec::with_capacity(self.size());   
    
        // Context
        data.push(T::context());
    
        // From vault
        data.extend_from_slice(self.from_vault.as_ref());
    
        // To vault
        data.extend_from_slice(self.to_vault.as_ref());
    
        // To account
        data.extend_from_slice(self.to_account.as_ref());
    
        // Units
        data.extend_from_slice(&self.u.to_be_bytes());

        // Variable payload
        self.variable_payload.try_encode(&mut data)?;
    
        Ok(Binary(data))

    }

    pub fn try_decode(
        data: Binary
    ) -> Result<CatalystV1Payload<T>, ContractError> {

        //TODO skip this check?
        let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;
        if context != &T::context() {
            return Err(ContractError::PayloadDecodingError {});
        }
    
        // From vault
        let from_vault = CatalystEncodedAddress::from_slice_unchecked(
            data.get(FROM_VAULT_START .. FROM_VAULT_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );
    
        // To vault
        let to_vault = CatalystEncodedAddress::from_slice_unchecked(
            data.get(TO_VAULT_START .. TO_VAULT_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );

        // To account
        let to_account = CatalystEncodedAddress::from_slice_unchecked(
            data.get(TO_ACCOUNT_START .. TO_ACCOUNT_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );
    
        // Units
        let u = U256::from_be_bytes(
            data.get(UNITS_START .. UNITS_END)
                .ok_or(ContractError::PayloadDecodingError {})?
                .try_into().unwrap()                            // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

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

    pub fn from_vault_as_string(
        &self
    ) -> Result<String, ContractError> {

        String::from_utf8(
            self.from_vault.try_decode()?
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn from_vault_validated(
        &self,
        deps: Deps
    ) -> Result<Addr, ContractError> {

        deps.api.addr_validate(
            &self.from_vault_as_string()?
        ).map_err(|err| err.into())

    }

    pub fn to_vault_as_string(
        &self
    ) -> Result<String, ContractError> {

        String::from_utf8(
            self.to_vault.try_decode()?
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn to_vault_validated(
        &self,
        deps: Deps
    ) -> Result<Addr, ContractError> {

        deps.api.addr_validate(
            &self.to_vault_as_string()?
        ).map_err(|err| err.into())

    }

    pub fn to_account_as_string(
        &self
    ) -> Result<String, ContractError> {

        String::from_utf8(
            self.to_account.try_decode()?
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn to_account_validated(
        &self,
        deps: Deps
    ) -> Result<Addr, ContractError> {

        deps.api.addr_validate(
            &self.to_account_as_string()?
        ).map_err(|err| err.into())

    }
}



// Variable Payloads ************************************************************************************************************
pub trait CatalystV1VariablePayload: Sized {

    fn context() -> u8;

    fn size(&self) -> usize;

    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError>;

    fn try_decode(buffer: Vec<u8>) -> Result<Self, ContractError>;

}


#[derive(Clone)]
pub struct CatalystCalldata {
    pub target: Addr,
    pub bytes: Binary
}


// Send asset payload
pub struct SendAssetVariablePayload {
    pub to_asset_index: u8,
    pub min_out: U256,
    pub from_amount: U256,
    pub from_asset: CatalystEncodedAddress,
    pub block_number: u32,
    pub calldata: Binary
}

impl SendAssetVariablePayload {

    pub fn from_asset_as_string(
        &self
    ) -> Result<String, ContractError> {
        
        String::from_utf8(
            self.from_asset.try_decode()?
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn min_out(
        &self
    ) -> Result<Uint128, ContractError> {

        Ok(
            self.min_out
                .try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?
        )

    }

    pub fn from_amount(
        &self
    ) -> Result<Uint128, ContractError> {

        Ok(
            self.from_amount
                .try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?
        )

    }

    pub fn parse_calldata(
        &self,
        deps: Deps
    ) -> Result<Option<CatalystCalldata>, ContractError> {
        
        parse_calldata(deps, self.calldata.0.clone())

    }

}

impl CatalystV1VariablePayload for SendAssetVariablePayload {

    fn context() -> u8 {
        CTX0_ASSET_SWAP
    }

    fn size(&self) -> usize {
        // Note: The following addition is way below the overflow threshold, and even if it were to overflow the code 
        // would still function properly, as this is just a runtime optimization.
        1                           // to asset index
        + 32                        // min out
        + 32                        // from amount
        + 65                        // from asset
        + 4                         // block number
        + 2                         // calldata length
        + self.calldata.len()       // calldata
    }

    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError> {

        // To asset index
        buffer.push(self.to_asset_index);
    
        // Min out
        buffer.extend_from_slice(&self.min_out.to_be_bytes());
    
        // From amount
        buffer.extend_from_slice(&self.from_amount.to_be_bytes());
    
        // From asset
        buffer.extend_from_slice(self.from_asset.as_ref());
    
        // Block number
        buffer.extend_from_slice(&self.block_number.to_be_bytes());
    
        // Calldata
        let calldata_length: u16 = self.calldata.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow
        buffer.extend_from_slice(&calldata_length.to_be_bytes());
        buffer.extend_from_slice(&self.calldata);

        Ok(())
    }

    fn try_decode(buffer: Vec<u8>) -> Result<Self, ContractError> {

        // To asset index
        let to_asset_index = buffer.get(CTX0_TO_ASSET_INDEX_POS)
            .ok_or(ContractError::PayloadDecodingError {})?.clone();

        // Min out
        let min_out = U256::from_be_bytes(
            buffer.get(
                CTX0_MIN_OUT_START .. CTX0_MIN_OUT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_MIN_OUT_START' and 'CTX0_MIN_OUT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // From amount
        let from_amount = U256::from_be_bytes(
            buffer.get(
                CTX0_FROM_AMOUNT_START .. CTX0_FROM_AMOUNT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_FROM_AMOUNT_START' and 'CTX0_FROM_AMOUNT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // From asset
        let from_asset = CatalystEncodedAddress::from_slice_unchecked(
            buffer.get(CTX0_FROM_ASSET_START .. CTX0_FROM_ASSET_END)
                .ok_or(ContractError::PayloadDecodingError {})?
        );
    
        // Block number
        let block_number = u32::from_be_bytes(
            buffer.get(
                CTX0_BLOCK_NUMBER_START .. CTX0_BLOCK_NUMBER_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_BLOCK_NUMBER_START' and 'CTX0_BLOCK_NUMBER_END' are 4 bytes apart, this should never panic //TODO overhaul
        );

        // Calldata
        let calldata_length: usize = u16::from_be_bytes(
            buffer.get(
                CTX0_DATA_LENGTH_START .. CTX0_DATA_LENGTH_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_DATA_LENGTH_START' and 'CTX0_DATA_LENGTH_END' are 2 bytes apart, this should never panic //TODO overhaul
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
            calldata
        })

    }

}


// Send liquidity payload
pub struct SendLiquidityVariablePayload {
    pub min_pool_tokens: U256,
    pub min_reference_asset: U256,
    pub from_amount: U256,
    pub block_number: u32,
    pub calldata: Binary
}

impl SendLiquidityVariablePayload {

    pub fn min_pool_tokens(
        &self
    ) -> Result<Uint128, ContractError> {

        Ok(
            self.min_pool_tokens
                .try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?
        )

    }

    //TODO overhaul - is the 'Uint128' type for this correct, or use U256?
    pub fn min_reference_asset(
        &self
    ) -> Result<Uint128, ContractError> {

        Ok(
            self.min_reference_asset
                .try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?
        )

    }

    pub fn from_amount(
        &self
    ) -> Result<Uint128, ContractError> {

        Ok(
            self.from_amount
                .try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?
        )

    }

    pub fn parse_calldata(
        &self,
        deps: Deps
    ) -> Result<Option<CatalystCalldata>, ContractError> {
        
        parse_calldata(deps, self.calldata.0.clone())

    }

}

impl CatalystV1VariablePayload for SendLiquidityVariablePayload {

    fn context() -> u8 {
        CTX1_LIQUIDITY_SWAP
    }

    fn size(&self) -> usize {
        // Note: The following addition is way below the overflow threshold, and even if it were to overflow the code 
        // would still function properly, as this is just a runtime optimization.
        32                          // min pool tokens
        + 32                        // min reference asset
        + 32                        // from amount
        + 4                         // block number
        + 2                         // calldata length
        + self.calldata.len()       // calldata
    }

    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError> {
    
        // Min out (pool tokens and reference amount)
        buffer.extend_from_slice(&self.min_pool_tokens.to_be_bytes());
        buffer.extend_from_slice(&self.min_reference_asset.to_be_bytes());
    
        // From amount
        buffer.extend_from_slice(&self.from_amount.to_be_bytes());
    
        // Block number
        buffer.extend_from_slice(&self.block_number.to_be_bytes());
    
        // Calldata
        let calldata_length: u16 = self.calldata.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow
        buffer.extend_from_slice(&calldata_length.to_be_bytes());
        buffer.extend_from_slice(&self.calldata);

        Ok(())
    }

    fn try_decode(buffer: Vec<u8>) -> Result<Self, ContractError> {

        // Min pool tokens
        let min_pool_tokens = U256::from_be_bytes(
            buffer.get(
                CTX1_MIN_POOL_TOKEN_START .. CTX1_MIN_POOL_TOKEN_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_MIN_POOL_TOKEN_START' and 'CTX1_MIN_POOL_TOKEN_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // Min reference asset
        let min_reference_asset = U256::from_be_bytes(
            buffer.get(
                CTX1_MIN_REFERENCE_START .. CTX1_MIN_REFERENCE_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_MIN_REFERENCE_START' and 'CTX1_MIN_REFERENCE_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // From amount
        let from_amount = U256::from_be_bytes(
            buffer.get(
                CTX1_FROM_AMOUNT_START .. CTX1_FROM_AMOUNT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_FROM_AMOUNT_START' and 'CTX1_FROM_AMOUNT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );
    
        // Block number
        let block_number = u32::from_be_bytes(
            buffer.get(
                CTX1_BLOCK_NUMBER_START .. CTX1_BLOCK_NUMBER_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_BLOCK_NUMBER_START' and 'CTX1_BLOCK_NUMBER_END' are 4 bytes apart, this should never panic //TODO overhaul
        );

        // Calldata
        let calldata_length: usize = u16::from_be_bytes(
            buffer.get(
                CTX1_DATA_LENGTH_START .. CTX1_DATA_LENGTH_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_DATA_LENGTH_START' and 'CTX1_DATA_LENGTH_END' are 2 bytes apart, this should never panic //TODO overhaul
        ) as usize;

        let calldata = Binary(
            buffer.get(
                CTX1_DATA_START ..
                CTX1_DATA_START + calldata_length
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
        );

        Ok(SendLiquidityVariablePayload {
            min_pool_tokens,
            min_reference_asset,
            from_amount,
            block_number,
            calldata
        })

    }
}




// Misc helpers *****************************************************************************************************************

fn parse_calldata(
    deps: Deps,
    calldata: Vec<u8>
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

// Wrapper around a bytes vec for encoding/decoding of 65-byte Catalyst payload addresses
pub struct CatalystEncodedAddress(Vec<u8>);

impl AsRef<[u8]> for CatalystEncodedAddress {
    fn as_ref(&self) -> &[u8] {
        &self.0.as_ref()
    }
}

impl Into<Vec<u8>> for CatalystEncodedAddress {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl Into<Binary> for CatalystEncodedAddress {
    fn into(self) -> Binary {
        Binary(self.0)
    }
}

impl TryFrom<Vec<u8>> for CatalystEncodedAddress {
    type Error = ContractError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {

        if value.len() != 65 {
            return Err(ContractError::PayloadDecodingError {})
        }

        Ok(Self(value))
    }
}

impl TryFrom<Binary> for CatalystEncodedAddress {
    type Error = ContractError;

    fn try_from(value: Binary) -> Result<Self, Self::Error> {

        if value.len() != 65 {
            return Err(ContractError::PayloadDecodingError {})
        }

        Ok(Self(value.0))
    }
}

impl TryFrom<&[u8]> for CatalystEncodedAddress {
    type Error = ContractError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {

        if value.len() != 65 {
            return Err(ContractError::PayloadDecodingError {})
        }

        Ok(Self(value.to_vec()))
    }
}


impl CatalystEncodedAddress {

    pub fn to_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn to_binary(self) -> Binary {
        Binary(self.0)
    }

    pub fn from_slice_unchecked(data: &[u8]) -> Self {
        // This method must only be used when it is guaranteed that data.len() is 65 bytes long
        Self(data.to_vec())
    }

    pub fn try_encode(address: &[u8]) -> Result<Self, ContractError> {

        let address_len = address.len();
        if address_len > 64 {
            return Err(ContractError::PayloadEncodingError {});
        }

        let mut encoded_address: Vec<u8> = Vec::with_capacity(65);
        encoded_address.push(address_len as u8);             // Casting to u8 is safe, as address_len is <= 64

        if address_len != 64 {
            encoded_address.extend_from_slice(vec![0u8; 64-address_len].as_ref());
        }

        encoded_address.extend_from_slice(address.as_ref());

        Ok(Self(encoded_address))
    }

    pub fn try_decode(&self) -> Result<Vec<u8>, ContractError> {

        let address_length: usize = *self.0.get(0)
            .ok_or(ContractError::PayloadDecodingError {})? as usize;

        self.0.get(65-address_length..)
            .ok_or(ContractError::PayloadDecodingError {})
            .map(|slice| slice.to_vec())

    }
}
