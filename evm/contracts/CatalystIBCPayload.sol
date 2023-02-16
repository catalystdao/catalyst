//SPDX-License-Identifier: UNLICENSED


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
//       + BLOCK_NUMBER     165 (4 bytes)
//       + SWAP_HASH        197 (32 bytes)
//
// Common Payload (end)
//    + DATA_LENGTH         LENGTH-N-2 (2 bytes)
//    + DATA                LENGTH-N   (N bytes)



// Contexts *********************************************************************************************************************

bytes1 constant CTX0_ASSET_SWAP     = 0x00;
bytes1 constant CTX1_LIQUIDITY_SWAP = 0x01;



// Type Sizes *******************************************************************************************************************

uint constant CONTEXT_SIZE        = 1;
uint constant FROM_POOL_SIZE      = 32;
uint constant TO_POOL_SIZE        = 32;
uint constant TO_ACCOUNT_SIZE     = 32;
uint constant UNITS_SIZE          = 32;

uint constant TO_ASSET_INDEX_SIZE = 1;
uint constant MIN_OUT_SIZE        = 32;
uint constant FROM_AMOUNT_SIZE    = 32;
uint constant FROM_ASSET_SIZE     = 32;
uint constant BLOCK_NUMBER_SIZE   = 4;
uint constant SWAP_HASH_SIZE      = 32;
uint constant DATA_LENGTH_SIZE    = 2;



// Common Payload ***************************************************************************************************************

uint constant CONTEXT_START       = 0;
uint constant CONTEXT_POS         = CONTEXT_START;
uint constant CONTEXT_END         = CONTEXT_START + CONTEXT_SIZE;

uint constant FROM_POOL_START     = CONTEXT_END;
uint constant FROM_POOL_END       = FROM_POOL_START + FROM_POOL_SIZE;

uint constant TO_POOL_START       = FROM_POOL_END;
uint constant TO_POOL_END         = TO_POOL_START + TO_POOL_SIZE;

uint constant TO_ACCOUNT_START    = TO_POOL_END;
uint constant TO_ACCOUNT_END      = TO_ACCOUNT_START + TO_ACCOUNT_SIZE;

uint constant UNITS_START         = TO_ACCOUNT_END;
uint constant UNITS_END           = UNITS_START + UNITS_SIZE;



// CTX0 Asset Swap Payload ******************************************************************************************************

uint constant CTX0_TO_ASSET_INDEX_START  = UNITS_END;
uint constant CTX0_TO_ASSET_INDEX_POS    = CTX0_TO_ASSET_INDEX_START;
uint constant CTX0_TO_ASSET_INDEX_END    = CTX0_TO_ASSET_INDEX_START + TO_ASSET_INDEX_SIZE;

uint constant CTX0_MIN_OUT_START         = CTX0_TO_ASSET_INDEX_END;
uint constant CTX0_MIN_OUT_END           = CTX0_MIN_OUT_START + MIN_OUT_SIZE;

uint constant CTX0_FROM_AMOUNT_START     = CTX0_MIN_OUT_END;
uint constant CTX0_FROM_AMOUNT_END       = CTX0_FROM_AMOUNT_START + FROM_AMOUNT_SIZE;

uint constant CTX0_FROM_ASSET_START      = CTX0_FROM_AMOUNT_END; 
uint constant CTX0_FROM_ASSET_END        = CTX0_FROM_ASSET_START + FROM_ASSET_SIZE;

uint constant CTX0_BLOCK_NUMBER_START    = CTX0_FROM_ASSET_END;
uint constant CTX0_BLOCK_NUMBER_END      = CTX0_BLOCK_NUMBER_START + BLOCK_NUMBER_SIZE;

uint constant CTX0_SWAP_HASH_START       = CTX0_BLOCK_NUMBER_END;
uint constant CTX0_SWAP_HASH_END         = CTX0_SWAP_HASH_START + SWAP_HASH_SIZE;

uint constant CTX0_DATA_LENGTH_START     = CTX0_SWAP_HASH_END;
uint constant CTX0_DATA_LENGTH_END       = CTX0_DATA_LENGTH_START + DATA_LENGTH_SIZE;

uint constant CTX0_DATA_START            = CTX0_DATA_LENGTH_END;



// CTX1 Liquidity Swap Payload **************************************************************************************************

uint constant CTX1_MIN_OUT_START         = UNITS_END;
uint constant CTX1_MIN_OUT_END           = CTX1_MIN_OUT_START + MIN_OUT_SIZE;

uint constant CTX1_FROM_AMOUNT_START     = CTX1_MIN_OUT_END;
uint constant CTX1_FROM_AMOUNT_END       = CTX1_FROM_AMOUNT_START + FROM_AMOUNT_SIZE;

uint constant CTX1_BLOCK_NUMBER_START    = CTX1_FROM_AMOUNT_END;
uint constant CTX1_BLOCK_NUMBER_END      = CTX1_BLOCK_NUMBER_START + BLOCK_NUMBER_SIZE;

uint constant CTX1_SWAP_HASH_START       = CTX1_BLOCK_NUMBER_END;
uint constant CTX1_SWAP_HASH_END         = CTX1_SWAP_HASH_START + SWAP_HASH_SIZE;

uint constant CTX1_DATA_LENGTH_START     = CTX1_SWAP_HASH_END;
uint constant CTX1_DATA_LENGTH_END       = CTX1_DATA_LENGTH_START + DATA_LENGTH_SIZE;

uint constant CTX1_DATA_START            = CTX1_DATA_LENGTH_END;