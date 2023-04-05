// Catalyst IBC payload structure ***********************************************************************************************
//
// Common Payload (beginning)
//    CONTEXT                 (1 byte)   
//    + FROM_POOL_LENGTH      (1 byte) 
//    + FROM_POOL             (FROM_POOL_LENGTH bytes) 
//    + TO_POOL_LENGTH        (1 byte) 
//    + TO_POOL               (TO_POOL_LENGTH bytes) 
//    + TO_ACCOUNT_LENGTH     (1 byte) 
//    + TO_ACCOUNT            (TO_ACCOUNT_LENGTH bytes) 
//    + UNITS                 (32 bytes) 
// 
// Context-depending Payload
//    CTX0 - 0x00 - Asset Swap Payload
//       + TO_ASSET_INDEX     (1 byte)
//       + MIN_OUT            (32 bytes)
//       + FROM_AMOUNT        (32 bytes)
//       + FROM_ASSET_LENGTH  (1 byte)
//       + FROM_ASSET         (FROM_ASSET_LENGTH bytes)
//       + BLOCK_NUMBER       (4 bytes)
//       + SWAP_HASH          (32 bytes)
//
//    CTX1 - 0x01 - Liquidity Swap Payload
//       + MIN_OUT            (32 bytes)
//       + FROM_AMOUNT        (32 bytes)
//       + BLOCK_NUMBER       (4 bytes)
//       + SWAP_HASH          (32 bytes)
//
// Common Payload (end)
//    + DATA_LENGTH           (2 bytes)
//    + DATA                  (DATA_LENGTH bytes)


// NOTE:
// The IBC payload contains several variable length parameters, hence it is not possible 
// to hardcode the exact position in which the different parameters are located at. The following 
// parameter positions/ranges are defined as if all variable length parameters had 0 length.


// Contexts *********************************************************************************************************************

pub const CTX0_ASSET_SWAP            : u8 = 0x00;
pub const CTX1_LIQUIDITY_SWAP        : u8 = 0x01;



// Common Payload ***************************************************************************************************************

pub const CONTEXT_POS                : usize = 0;

pub const FROM_POOL_LENGTH_POS       : usize = 1;
pub const FROM_POOL_START            : usize = 2;

pub const TO_POOL_LENGTH_POS         : usize = 2;
pub const TO_POOL_START              : usize = 3;

pub const TO_ACCOUNT_POS             : usize = 3;
pub const TO_ACCOUNT_START           : usize = 4;

pub const UNITS_START                : usize = 4;
pub const UNITS_END                  : usize = 36;



// CTX0 Asset Swap Payload ******************************************************************************************************

pub const CTX0_TO_ASSET_INDEX_POS    : usize = 36;

pub const CTX0_MIN_OUT_START         : usize = 37;
pub const CTX0_MIN_OUT_END           : usize = 69;

pub const CTX0_FROM_AMOUNT_START     : usize = 69;
pub const CTX0_FROM_AMOUNT_END       : usize = 101;

pub const CTX0_FROM_ASSET_POS        : usize = 101; 
pub const CTX0_FROM_ASSET_START      : usize = 102;

pub const CTX0_BLOCK_NUMBER_START    : usize = 102;
pub const CTX0_BLOCK_NUMBER_END      : usize = 106;

pub const CTX0_SWAP_HASH_START       : usize = 106;
pub const CTX0_SWAP_HASH_END         : usize = 138;

pub const CTX0_DATA_LENGTH_START     : usize = 138;
pub const CTX0_DATA_LENGTH_END       : usize = 140;

pub const CTX0_DATA_START            : usize = 140;



// CTX1 Liquidity Swap Payload **************************************************************************************************

pub const CTX1_MIN_OUT_START         : usize = 36;
pub const CTX1_MIN_OUT_END           : usize = 68;

pub const CTX1_FROM_AMOUNT_START     : usize = 68;
pub const CTX1_FROM_AMOUNT_END       : usize = 100;

pub const CTX1_BLOCK_NUMBER_START    : usize = 100;
pub const CTX1_BLOCK_NUMBER_END      : usize = 104;

pub const CTX1_SWAP_HASH_START       : usize = 104;
pub const CTX1_SWAP_HASH_END         : usize = 136;

pub const CTX1_DATA_LENGTH_START     : usize = 136;
pub const CTX1_DATA_LENGTH_END       : usize = 138;

pub const CTX1_DATA_START            : usize = 138;



// Helpers **********************************************************************************************************************
use cosmwasm_std::{Deps, Addr, Uint128};
use ethnum::U256;
use crate::ContractError;

pub trait CatalystV1VariablePayload<'a>: Sized {

    fn context() -> u8;

    fn size(&self) -> usize;

    fn try_encode(&self, buffer: &mut Vec<u8>) -> Result<(), ContractError>;

    fn try_decode(buffer: &'a Vec<u8>, offset: usize) -> Result<Self, ContractError>;

}

pub enum CatalystV1Packet<'a> {
    SendAsset(CatalystV1SendAssetPayload<'a>),
    SendLiquidity(CatalystV1SendLiquidityPayload<'a>)
}

impl<'a> CatalystV1Packet<'a> {
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
        data: &'a Vec<u8>
    ) -> Result<CatalystV1Packet<'a>, ContractError> {

        let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;

        match *context {
            CTX0_ASSET_SWAP => Ok(CatalystV1Packet::SendAsset(
                CatalystV1SendAssetPayload::try_decode(&data)?
            )),
            CTX1_LIQUIDITY_SWAP => Ok(CatalystV1Packet::SendLiquidity(
                CatalystV1SendLiquidityPayload::try_decode(&data)?
            )),
            _ => return Err(ContractError::PayloadDecodingError {})
        }

    }
}

pub type CatalystV1SendAssetPayload<'a> = CatalystV1Payload<'a, SendAssetVariablePayload<'a>>;
pub type CatalystV1SendLiquidityPayload<'a> = CatalystV1Payload<'a, SendLiquidityVariablePayload<'a>>;

pub struct CatalystV1Payload<'a, T: CatalystV1VariablePayload<'a>> {
    pub from_pool: &'a [u8],
    pub to_pool: &'a [u8],
    pub to_account: &'a [u8],
    pub u: U256,
    pub variable_payload: T
}

impl<'a, T: CatalystV1VariablePayload<'a>> CatalystV1Payload<'a, T> {

    pub fn try_encode(
        &self
    ) -> Result<Vec<u8>, ContractError> {

        // Preallocate the required size for the IBC payload to avoid runtime reallocations.
        let mut data: Vec<u8> = Vec::with_capacity(
            self.from_pool.len()
            + self.to_pool.len()
            + self.to_account.len()
            + self.variable_payload.size()
        );   // Addition is way below the overflow threshold, and even if it were to overflow the code would still function properly, as this is just a runtime optimization.
    
        // Context
        data.push(T::context());
    
        // From pool
        data.push(
            self.from_pool.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
        );
        data.extend_from_slice(&self.from_pool);
    
        // To pool
        data.push(
            self.to_pool.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
        );
        data.extend_from_slice(&self.to_pool);
    
        // To account
        data.push(
            self.to_account.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
        );
        data.extend_from_slice(&self.to_account);
    
        // Units
        data.extend_from_slice(&self.u.to_be_bytes());

        // Variable payload
        self.variable_payload.try_encode(&mut data)?;
    
        Ok(data)

    }

    pub fn try_decode(
        data: &'a Vec<u8>
    ) -> Result<CatalystV1Payload<'a, T>, ContractError> {

        //TODO skip this check?
        let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;
        if context != &T::context() {
            return Err(ContractError::PayloadDecodingError {});
        }
    
        let mut offset: usize = 0;
    
        // From pool
        let from_pool_length: usize = *data.get(FROM_POOL_LENGTH_POS)
            .ok_or(ContractError::PayloadDecodingError {})? as usize;
    
        let from_pool = data.get(
            FROM_POOL_START ..
            FROM_POOL_START + from_pool_length
        ).ok_or(ContractError::PayloadDecodingError {})?;
        
        offset += from_pool_length;
    
    
        // To pool
        let to_pool_length: usize = *data.get(TO_POOL_LENGTH_POS + offset)
            .ok_or(ContractError::PayloadDecodingError {})? as usize;
    
        let to_pool = data.get(
            TO_POOL_START + offset ..
            TO_POOL_START + offset + to_pool_length
        ).ok_or(ContractError::PayloadDecodingError {})?;
        
        offset += to_pool_length;
    
    
        // To account
        let to_account_length: usize = *data.get(TO_ACCOUNT_POS + offset)
            .ok_or(ContractError::PayloadDecodingError {})? as usize;
    
        let to_account = data.get(
            TO_ACCOUNT_START + offset ..
            TO_ACCOUNT_START + offset + to_account_length
        ).ok_or(ContractError::PayloadDecodingError {})?;
        
        offset += to_account_length;
    
    
        // Units
        let u = U256::from_be_bytes(
            data.get(
                UNITS_START + offset .. UNITS_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        let variable_payload = T::try_decode(data, offset)?;

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


pub struct SendAssetVariablePayload<'a> {
    pub to_asset_index: u8,
    pub min_out: U256,
    pub from_amount: U256,
    pub from_asset: &'a [u8],
    pub block_number: u32,
    pub swap_hash: &'a [u8],
    pub calldata: Vec<u8>
}

impl<'a> CatalystV1VariablePayload<'a> for SendAssetVariablePayload<'a> {

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
        buffer.push(
            self.from_asset.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
        );
        buffer.extend_from_slice(self.from_asset);
    
        // Block number
        buffer.extend_from_slice(&self.block_number.to_be_bytes());
    
        // Swap hash
        if self.swap_hash.len() != 32 {
            return Err(ContractError::PayloadEncodingError {});
        }
        buffer.extend_from_slice(&self.swap_hash);
    
        // Calldata
        let calldata_length: u16 = self.calldata.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow
        buffer.extend_from_slice(&calldata_length.to_be_bytes());
        buffer.extend_from_slice(&self.calldata);

        Ok(())
    }

    fn try_decode(buffer: &'a Vec<u8>, mut offset: usize) -> Result<Self, ContractError> {

        // To asset index
        let to_asset_index = buffer.get(CTX0_TO_ASSET_INDEX_POS + offset)
            .ok_or(ContractError::PayloadDecodingError {})?.clone();

        // Min out
        let min_out = U256::from_be_bytes(
            buffer.get(
                CTX0_MIN_OUT_START + offset .. CTX0_MIN_OUT_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_MIN_OUT_START' and 'CTX0_MIN_OUT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // From amount
        let from_amount = U256::from_be_bytes(
            buffer.get(
                CTX0_FROM_AMOUNT_START + offset .. CTX0_FROM_AMOUNT_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_FROM_AMOUNT_START' and 'CTX0_FROM_AMOUNT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // From asset
        let from_asset_length: usize = *buffer.get(CTX0_FROM_ASSET_POS + offset)
            .ok_or(ContractError::PayloadDecodingError {})? as usize;
        offset += from_asset_length;
    
        let from_asset = buffer.get(
            CTX0_FROM_ASSET_START + offset ..
            CTX0_FROM_ASSET_START + offset + from_asset_length
        ).ok_or(ContractError::PayloadDecodingError {})?;
    
        // Block number
        let block_number = u32::from_be_bytes(
            buffer.get(
                CTX0_BLOCK_NUMBER_START + offset .. CTX0_BLOCK_NUMBER_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_BLOCK_NUMBER_START' and 'CTX0_BLOCK_NUMBER_END' are 4 bytes apart, this should never panic //TODO overhaul
        );

        // Swap hash
        let swap_hash = buffer.get(
            CTX0_SWAP_HASH_START + offset .. CTX0_SWAP_HASH_END + offset
        ).ok_or(ContractError::PayloadDecodingError {})?;

        // Calldata
        let calldata_length: usize = u16::from_be_bytes(
            buffer.get(
                CTX0_DATA_LENGTH_START + offset .. CTX0_DATA_LENGTH_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX0_DATA_LENGTH_START' and 'CTX0_DATA_LENGTH_END' are 2 bytes apart, this should never panic //TODO overhaul
        ) as usize;

        let calldata = buffer.get(
            CTX0_DATA_START + offset ..
            CTX0_DATA_START + offset + calldata_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();

        Ok(SendAssetVariablePayload::<'a> {
            to_asset_index,
            min_out,
            from_amount,
            from_asset,
            swap_hash,
            block_number,
            calldata
        })

    }

}

impl<'a> SendAssetVariablePayload<'a> {

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


pub struct SendLiquidityVariablePayload<'a> {
    pub min_out: U256,
    pub from_amount: U256,
    pub block_number: u32,
    pub swap_hash: &'a [u8],
    pub calldata: Vec<u8>
}

impl<'a> CatalystV1VariablePayload<'a> for SendLiquidityVariablePayload<'a> {

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
    
        // Swap hash
        if self.swap_hash.len() != 32 {
            return Err(ContractError::PayloadEncodingError {});
        }
        buffer.extend_from_slice(&self.swap_hash);
    
        // Calldata
        let calldata_length: u16 = self.calldata.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow
        buffer.extend_from_slice(&calldata_length.to_be_bytes());
        buffer.extend_from_slice(&self.calldata);

        Ok(())
    }

    fn try_decode(buffer: &'a Vec<u8>, offset: usize) -> Result<Self, ContractError> {

        // Min out
        let min_out = U256::from_be_bytes(
            buffer.get(
                CTX1_MIN_OUT_START + offset .. CTX1_MIN_OUT_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_MIN_OUT_START' and 'CTX1_MIN_OUT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );

        // From amount
        let from_amount = U256::from_be_bytes(
            buffer.get(
                CTX1_FROM_AMOUNT_START + offset .. CTX1_FROM_AMOUNT_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_FROM_AMOUNT_START' and 'CTX1_FROM_AMOUNT_END' are 32 bytes apart, this should never panic //TODO overhaul
        );
    
        // Block number
        let block_number = u32::from_be_bytes(
            buffer.get(
                CTX1_BLOCK_NUMBER_START + offset .. CTX1_BLOCK_NUMBER_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_BLOCK_NUMBER_START' and 'CTX1_BLOCK_NUMBER_END' are 4 bytes apart, this should never panic //TODO overhaul
        );

        // Swap hash
        let swap_hash = buffer.get(
            CTX1_SWAP_HASH_START + offset .. CTX1_SWAP_HASH_END + offset
        ).ok_or(ContractError::PayloadDecodingError {})?;

        // Calldata
        let calldata_length: usize = u16::from_be_bytes(
            buffer.get(
                CTX1_DATA_LENGTH_START + offset .. CTX1_DATA_LENGTH_END + offset
            ).ok_or(
                ContractError::PayloadDecodingError {}
            )?.try_into().unwrap()                          // If 'CTX1_DATA_LENGTH_START' and 'CTX1_DATA_LENGTH_END' are 2 bytes apart, this should never panic //TODO overhaul
        ) as usize;

        let calldata = buffer.get(
            CTX1_DATA_START + offset ..
            CTX1_DATA_START + offset + calldata_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();

        Ok(SendLiquidityVariablePayload {
            min_out,
            from_amount,
            swap_hash,
            block_number,
            calldata
        })

    }
}

impl<'a> SendLiquidityVariablePayload<'a> {

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
