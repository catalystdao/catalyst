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