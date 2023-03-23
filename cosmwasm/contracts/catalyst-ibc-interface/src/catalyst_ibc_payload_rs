// Catalyst IBC payload structure ***********************************************************************************************
//
// Common Payload (beginning)
//    CONTEXT               0   (1 byte)   
//    + FROM_POOL           1   (32 bytes) 
//    + TO_POOL             33  (32 bytes) 
//    + TO_ACCOUNT          65  (32 bytes) 
//    + UNITS               97  (32 bytes) 
// 
// Context-depending Payload
//    CTX0 - 0x00 - Asset Swap Payload
//       + TO_ASSET_INDEX   129 (1 byte)
//       + MIN_OUT          130 (32 bytes)
//       + FROM_AMOUNT      162 (32 bytes)
//       + FROM_ASSET       194 (32 bytes)
//       + BLOCK_NUMBER     226 (4 bytes)
//       + SWAP_HASH        230 (32 bytes)
//
//    CTX1 - 0x01 - Liquidity Swap Payload
//       + MIN_OUT          129 (32 bytes)
//       + FROM_AMOUNT      161 (32 bytes)
//       + BLOCK_NUMBER     193 (4 bytes)
//       + SWAP_HASH        197 (32 bytes)
//
// Common Payload (end)
//    + DATA_LENGTH         LENGTH-N-2 (2 bytes)
//    + DATA                LENGTH-N   (N bytes)



// Contexts *********************************************************************************************************************

pub const CTX0_ASSET_SWAP     : u8 = 0x00;
pub const CTX1_LIQUIDITY_SWAP : u8 = 0x01;



// Common Payload ***************************************************************************************************************

pub const CONTEXT_POS         : usize = 0;

pub const FROM_POOL_START     : usize = 1;
pub const FROM_POOL_END       : usize = 33;

pub const TO_POOL_START       : usize = 33;
pub const TO_POOL_END         : usize = 65;

pub const TO_ACCOUNT_START    : usize = 65;
pub const TO_ACCOUNT_END      : usize = 97;

pub const UNITS_START         : usize = 97;
pub const UNITS_END           : usize = 129;



// CTX0 Asset Swap Payload ******************************************************************************************************

pub const CTX0_TO_ASSET_INDEX_POS    : usize = 129;

pub const CTX0_MIN_OUT_START         : usize = 130;
pub const CTX0_MIN_OUT_END           : usize = 162;

pub const CTX0_FROM_AMOUNT_START     : usize = 162;
pub const CTX0_FROM_AMOUNT_END       : usize = 194;

pub const CTX0_FROM_ASSET_START      : usize = 194; 
pub const CTX0_FROM_ASSET_END        : usize = 226;

pub const CTX0_BLOCK_NUMBER_START    : usize = 226;
pub const CTX0_BLOCK_NUMBER_END      : usize = 230;

pub const CTX0_SWAP_HASH_START       : usize = 230;
pub const CTX0_SWAP_HASH_END         : usize = 262;

pub const CTX0_DATA_LENGTH_START     : usize = 262;
pub const CTX0_DATA_LENGTH_END       : usize = 264;

pub const CTX0_DATA_START            : usize = 264;



// CTX1 Liquidity Swap Payload **************************************************************************************************

pub const CTX1_MIN_OUT_START         : usize = 129;
pub const CTX1_MIN_OUT_END           : usize = 161;

pub const CTX1_FROM_AMOUNT_START     : usize = 161;
pub const CTX1_FROM_AMOUNT_END       : usize = 193;

pub const CTX1_BLOCK_NUMBER_START    : usize = 193;
pub const CTX1_BLOCK_NUMBER_END      : usize = 197;

pub const CTX1_SWAP_HASH_START       : usize = 197;
pub const CTX1_SWAP_HASH_END         : usize = 229;

pub const CTX1_DATA_LENGTH_START     : usize = 229;
pub const CTX1_DATA_LENGTH_END       : usize = 231;

pub const CTX1_DATA_START            : usize = 231;