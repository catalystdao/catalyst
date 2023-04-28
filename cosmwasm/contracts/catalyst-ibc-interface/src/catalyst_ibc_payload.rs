
// ******************************************************************************************************************************
// Catalyst IBC payload structure
// ******************************************************************************************************************************

// Common Payload (beginning)
//    CONTEXT                   0   (1 byte)
//    + FROM_POOL               1   (65 bytes)
//    + TO_POOL                 66  (65 bytes)
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

pub const FROM_POOL_START            : usize = 1;
pub const FROM_POOL_END              : usize = 66;

pub const TO_POOL_START              : usize = 66;
pub const TO_POOL_END                : usize = 131;

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

pub const CTX1_MIN_OUT_START         : usize = 228;
pub const CTX1_MIN_OUT_END           : usize = 260;

pub const CTX1_FROM_AMOUNT_START     : usize = 260;
pub const CTX1_FROM_AMOUNT_END       : usize = 292;

pub const CTX1_BLOCK_NUMBER_START    : usize = 292;
pub const CTX1_BLOCK_NUMBER_END      : usize = 296;

pub const CTX1_DATA_LENGTH_START     : usize = 296;
pub const CTX1_DATA_LENGTH_END       : usize = 298;

pub const CTX1_DATA_START            : usize = 298;





// ******************************************************************************************************************************
// Payload Helpers
// ******************************************************************************************************************************
use cosmwasm_std::{Deps, Addr, Uint128};
use ethnum::U256;
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
    ) -> Result<Vec<u8>, ContractError> {
        
        match self {
            CatalystV1Packet::SendAsset(payload) => payload.try_encode(),
            CatalystV1Packet::SendLiquidity(payload) => payload.try_encode(),
        }

    }

    //TODO pass the 'IbcPacket' to try_decode and return a copy of the data and not references to it?
    //TODO Or make CatalystV1SendAssetPayload/CatalystV1SendLiquidityPayload accept the IBC packet directly (rename them to CatalystV1SendAssetPACKET)
    pub fn try_decode(
        data: Vec<u8>
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
    pub from_pool: Vec<u8>,
    pub to_pool: Vec<u8>,
    pub to_account: Vec<u8>,
    pub u: U256,
    pub variable_payload: T
}

impl<T: CatalystV1VariablePayload> CatalystV1Payload<T> {

    pub fn size(&self) -> usize {
        // Addition is way below the overflow threshold
        1                                   // Context
        + 65                                // From pool
        + 65                                // To pool
        + 65                                // To account
        + 32                                // Units
        + self.variable_payload.size()
    }

    pub fn try_encode(
        &self
    ) -> Result<Vec<u8>, ContractError> {

        // Preallocate the required size for the IBC payload to avoid runtime reallocations.
        let mut data: Vec<u8> = Vec::with_capacity(self.size());   
    
        // Context
        data.push(T::context());
    
        // From pool
        data.extend_from_slice(
            encode_address(self.from_pool.as_ref())?.as_ref()
        );
    
        // To pool
        data.extend_from_slice(
            encode_address(self.to_pool.as_ref())?.as_ref()
        );
    
        // To account
        data.extend_from_slice(
            encode_address(self.to_account.as_ref())?.as_ref()
        );
    
        // Units
        data.extend_from_slice(&self.u.to_be_bytes());

        // Variable payload
        self.variable_payload.try_encode(&mut data)?;
    
        Ok(data)

    }

    pub fn try_decode(
        data: Vec<u8>
    ) -> Result<CatalystV1Payload<T>, ContractError> {

        //TODO skip this check?
        let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;
        if context != &T::context() {
            return Err(ContractError::PayloadDecodingError {});
        }
    
        // From pool
        let from_pool = decode_address(
            data.get(FROM_POOL_START .. FROM_POOL_END).ok_or(ContractError::PayloadDecodingError {})?
        )?;
    
        // To pool
        let to_pool = decode_address(
            data.get(TO_POOL_START .. TO_POOL_END).ok_or(ContractError::PayloadDecodingError {})?
        )?;
    
        // To account
        let to_account = decode_address(
            data.get(TO_ACCOUNT_START .. TO_ACCOUNT_END).ok_or(ContractError::PayloadDecodingError {})?
        )?;
    
        // Units
        let u = U256::from_be_bytes(
            data.get(UNITS_START .. UNITS_END)
                .ok_or(ContractError::PayloadDecodingError {})?
                .try_into().unwrap()                            // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        let variable_payload = T::try_decode(data)?;

        return Ok(
            Self {
                from_pool,
                to_pool,
                to_account,
                u,
                variable_payload
            }
        )
    

    }

    pub fn from_pool_unsafe_string(
        &self
    ) -> Result<String, ContractError> {

        String::from_utf8(
            self.from_pool.to_vec()
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn from_pool(
        &self,
        deps: Deps
    ) -> Result<Addr, ContractError> {

        deps.api.addr_validate(
            &self.from_pool_unsafe_string()?
        ).map_err(|err| err.into())

    }

    pub fn to_pool_unsafe_string(
        &self
    ) -> Result<String, ContractError> {

        String::from_utf8(
            self.to_pool.to_vec()
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn to_pool(
        &self,
        deps: Deps
    ) -> Result<Addr, ContractError> {

        deps.api.addr_validate(
            &self.to_pool_unsafe_string()?
        ).map_err(|err| err.into())

    }

    pub fn to_account_unsafe_string(
        &self
    ) -> Result<String, ContractError> {

        String::from_utf8(
            self.to_account.to_vec()
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn to_account(
        &self,
        deps: Deps
    ) -> Result<Addr, ContractError> {

        deps.api.addr_validate(
            &self.to_account_unsafe_string()?
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


// Send asset payload
pub struct SendAssetVariablePayload {
    pub to_asset_index: u8,
    pub min_out: U256,
    pub from_amount: U256,
    pub from_asset: Vec<u8>,
    pub block_number: u32,
    pub calldata: Vec<u8>
}

impl SendAssetVariablePayload {

    pub fn from_asset_unsafe_string(
        &self
    ) -> Result<String, ContractError> {
        
        String::from_utf8(
            self.from_asset.to_vec()
        ).map_err(|_| ContractError::PayloadDecodingError {})

    }

    pub fn min_out(
        &self
    ) -> Result<Uint128, ContractError> {

        let min_out = self.min_out;
        // For CosmWasm pools, min_out should be Uint128
        if min_out > U256::from(Uint128::MAX.u128()) {             //TODO overhaul - more efficient way to do this?
            return Err(ContractError::PayloadDecodingError {});
        }
        Ok(Uint128::from(min_out.as_u128()))

    }

    pub fn from_amount(
        &self
    ) -> Result<Uint128, ContractError> {

        let from_amount = self.from_amount;
        // For CosmWasm pools, from_amount should be Uint128
        if from_amount > U256::from(Uint128::MAX.u128()) {             //TODO overhaul - more efficient way to do this?
            return Err(ContractError::PayloadDecodingError {});
        }
        Ok(Uint128::from(from_amount.as_u128()))

    }

}

impl CatalystV1VariablePayload for SendAssetVariablePayload {

    fn context() -> u8 {
        CTX0_ASSET_SWAP
    }

    fn size(&self) -> usize {
        CTX0_DATA_START             // This defines the size of all the fixed-length elements of the payload
        + self.from_asset.len()
        + self.calldata.len()       // Addition is way below the overflow threshold, and even if it were to overflow the code would still function properly, as this is just a runtime optimization.
    }

    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError> {

        // To asset index
        buffer.push(self.to_asset_index);
    
        // Min out
        buffer.extend_from_slice(&self.min_out.to_be_bytes());
    
        // From amount
        buffer.extend_from_slice(&self.from_amount.to_be_bytes());
    
        // From asset
        buffer.extend_from_slice(
            encode_address(self.from_asset.as_ref())?.as_ref()
        );
    
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
        let from_asset = decode_address(
            buffer.get(CTX0_FROM_ASSET_START .. CTX0_FROM_ASSET_END).ok_or(ContractError::PayloadDecodingError {})?
        )?;
    
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

        let calldata = buffer.get(
            CTX0_DATA_START .. CTX0_DATA_START + calldata_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();

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
    pub min_out: U256,
    pub from_amount: U256,
    pub block_number: u32,
    pub calldata: Vec<u8>
}

impl SendLiquidityVariablePayload {

    pub fn min_out(
        &self
    ) -> Result<Uint128, ContractError> {

        let min_out = self.min_out;
        // For CosmWasm pools, min_out should be Uint128
        if min_out > U256::from(Uint128::MAX.u128()) {             //TODO overhaul - more efficient way to do this?
            return Err(ContractError::PayloadDecodingError {});
        }
        Ok(Uint128::from(min_out.as_u128()))

    }

    pub fn from_amount(
        &self
    ) -> Result<Uint128, ContractError> {

        let from_amount = self.from_amount;
        // For CosmWasm pools, from_amount should be Uint128
        if from_amount > U256::from(Uint128::MAX.u128()) {             //TODO overhaul - more efficient way to do this?
            return Err(ContractError::PayloadDecodingError {});
        }
        Ok(Uint128::from(from_amount.as_u128()))

    }

}

impl CatalystV1VariablePayload for SendLiquidityVariablePayload {

    fn context() -> u8 {
        CTX1_LIQUIDITY_SWAP
    }

    fn size(&self) -> usize {
        CTX1_DATA_START             // This defines the size of all the fixed-length elements of the payload
        + self.calldata.len()       // Addition is way below the overflow threshold, and even if it were to overflow the code would still function properly, as this is just a runtime optimization.
    }

    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError> {
    
        // Min out
        buffer.extend_from_slice(&self.min_out.to_be_bytes());
    
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

        // Min out
        let min_out = U256::from_be_bytes(
            buffer.get(
                CTX1_MIN_OUT_START .. CTX1_MIN_OUT_END
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_MIN_OUT_START' and 'CTX1_MIN_OUT_END' are 32 bytes apart, this should never panic //TODO overhaul
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

        let calldata = buffer.get(
            CTX1_DATA_START ..
            CTX1_DATA_START + calldata_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();

        Ok(SendLiquidityVariablePayload {
            min_out,
            from_amount,
            block_number,
            calldata
        })

    }
}




// Misc helpers *****************************************************************************************************************

fn encode_address(address: &[u8]) -> Result<Vec<u8>, ContractError> {

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

    Ok(encoded_address)
}

fn decode_address(payload: &[u8]) -> Result<Vec<u8>, ContractError> {

    if payload.len() != 65 {
        return Err(ContractError::PayloadDecodingError {})
    }

    let address_length: usize = *payload.get(0)
        .ok_or(ContractError::PayloadDecodingError {})? as usize;

    payload.get(65-address_length..)
        .ok_or(ContractError::PayloadDecodingError {})
        .map(|slice| slice.to_vec())

}