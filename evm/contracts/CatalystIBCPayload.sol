//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

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

bytes1 constant CTX0_ASSET_SWAP     = 0x00;
bytes1 constant CTX1_LIQUIDITY_SWAP = 0x01;



// Common Payload ***************************************************************************************************************

uint constant CONTEXT_POS         = 0;

uint constant FROM_POOL_START     = 1;
uint constant FROM_POOL_END       = 33;

uint constant TO_POOL_START       = 33;
uint constant TO_POOL_END         = 65;

uint constant TO_ACCOUNT_START    = 65;
uint constant TO_ACCOUNT_END      = 97;

uint constant UNITS_START         = 97;
uint constant UNITS_END           = 129;



// CTX0 Asset Swap Payload ******************************************************************************************************

uint constant CTX0_TO_ASSET_INDEX_POS    = 129;

uint constant CTX0_MIN_OUT_START         = 130;
uint constant CTX0_MIN_OUT_END           = 162;

uint constant CTX0_FROM_AMOUNT_START     = 162;
uint constant CTX0_FROM_AMOUNT_END       = 194;

uint constant CTX0_FROM_ASSET_START      = 194; 
uint constant CTX0_FROM_ASSET_END        = 226;

uint constant CTX0_BLOCK_NUMBER_START    = 226;
uint constant CTX0_BLOCK_NUMBER_END      = 230;

uint constant CTX0_SWAP_HASH_START       = 230;
uint constant CTX0_SWAP_HASH_END         = 262;

uint constant CTX0_DATA_LENGTH_START     = 262;
uint constant CTX0_DATA_LENGTH_END       = 264;

uint constant CTX0_DATA_START            = 264;



// CTX1 Liquidity Swap Payload **************************************************************************************************

uint constant CTX1_MIN_POOL_TOKEN_START  = 129;
uint constant CTX1_MIN_POOL_TOKEN_END    = 161;

uint constant CTX1_MIN_REFERENCE_START   = 161;
uint constant CTX1_MIN_REFERENCE_END     = 193;

uint constant CTX1_FROM_AMOUNT_START     = 193;
uint constant CTX1_FROM_AMOUNT_END       = 225;

uint constant CTX1_BLOCK_NUMBER_START    = 225;
uint constant CTX1_BLOCK_NUMBER_END      = 229;

uint constant CTX1_SWAP_HASH_START       = 229;
uint constant CTX1_SWAP_HASH_END         = 261;

uint constant CTX1_DATA_LENGTH_START     = 261;
uint constant CTX1_DATA_LENGTH_END       = 263;

uint constant CTX1_DATA_START            = 263;