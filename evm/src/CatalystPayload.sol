//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

// Catalyst IBC payload structure ***********************************************************************************************
// Note: Addresses have 65 bytes reserved, however, the first byte should only be used for the address size.
//
// Common Payload (beginning)
//    CONTEXT               0   (1 byte)
//    + FROM_VAULT_LENGTH   1   (1 byte)
//    + FROM_VAULT          2   (64 bytes)
//    + TO_VAULT_LENGTH     66  (1 byte)
//    + TO_VAULT            67  (64 bytes)
//    + TO_ACCOUNT_LENGTH   131 (1 byte)
//    + TO_ACCOUNT          132 (64 bytes)
//    + UNITS               196 (32 bytes)
// 
// Context-depending Payload
//    CTX0 - 0x00 - Asset Swap Payload
//       + TO_ASSET_INDEX   228 (1 byte)
//       + MIN_OUT          229 (32 bytes)
//       + FROM_AMOUNT      261 (32 bytes)
//       + FROM_ASSET_LEN   293 (1 byte)
//       + FROM_ASSET       294 (64 bytes)
//       + BLOCK_NUMBER     358 (4 bytes)
//
//    CTX1 - 0x01 - Liquidity Swap Payload
//       + MIN_OUT          228 (32 bytes)
//       + MIN_REFERENCE    260 (32 bytes)
//       + FROM_AMOUNT      292 (32 bytes)
//       + BLOCK_NUMBER     324 (4 bytes)
//
//    CTX2 - 0x02 - Please Underwrite Payload
//      (Non-underwrite logic)
//       + TO_ASSET_INDEX   228 (1 byte)
//       + MIN_OUT          229 (32 bytes)
//       + FROM_AMOUNT      261 (32 bytes)
//       + FROM_ASSET_LEN   293 (1 byte)
//       + FROM_ASSET       294 (64 bytes)
//       + BLOCK_NUMBER     358 (4 bytes)
//      (Fallback Logic)
//       + TO_ACCOUNT_FALLBACK_LEN 262 (1 byte)
//       + TO_ACCOUNT_FALLBACK     263 (64 bytes)
//      (Underwrite Logic)
//       + UW_INCENTIVE     327 (2 bytes)
//
//    CTX3 - 0x03 - Purpose Underwrite Payload
//      (Fallback Logic)
//       + TO_ASSET_INDEX   228 (1 byte)
//      (Escrow Logic)
//       + FROM_AMOUNT      229 (32 bytes)
//       + FROM_ASSET_LEN   261 (1 byte)
//       + FROM_ASSET       262 (64 bytes)
//       + BLOCK_NUMBER     226 (4 bytes)
//      (Underwrite Logic)
//       + UNDERWRITE_ID    330 (32 bytes)
// 
// Common Payload (end)
//    + DATA_LENGTH         LENGTH-N-2 (2 bytes)
//    + DATA                LENGTH-N   (N bytes)



// Contexts *********************************************************************************************************************

bytes1 constant CTX0_ASSET_SWAP     = 0x00;
bytes1 constant CTX1_LIQUIDITY_SWAP = 0x01;
bytes1 constant CTX2_ASSET_SWAP_PLEASE_UNDERWRITE = 0x02;
bytes1 constant CTX3_ASSET_SWAP_PURPOSE_UNDERWRITE = 0x03;



// Common Payload ***************************************************************************************************************

uint constant CONTEXT_POS           = 0;

uint constant FROM_VAULT_LENGTH_POS = 1;
uint constant FROM_VAULT_START      = 2;
uint constant FROM_VAULT_START_EVM  = 46;  // If the address is an EVM address, this is the start
uint constant FROM_VAULT_END        = 66;

uint constant TO_VAULT_LENGTH_POS   = 66;
uint constant TO_VAULT_START        = 67;
uint constant TO_VAULT_START_EVM    = 111;  // If the address is an EVM address, this is the start
uint constant TO_VAULT_END          = 131;

uint constant TO_ACCOUNT_LENGTH_POS = 131;
uint constant TO_ACCOUNT_START      = 132;
uint constant TO_ACCOUNT_START_EVM  = 176;  // If the address is an EVM address, this is the start
uint constant TO_ACCOUNT_END        = 196;

uint constant UNITS_START           = 196;
uint constant UNITS_END             = 228;



// CTX0 Asset Swap Payload ******************************************************************************************************

uint constant CTX0_TO_ASSET_INDEX_POS    = 228;

uint constant CTX0_MIN_OUT_START         = 229;
uint constant CTX0_MIN_OUT_END           = 261;

uint constant CTX0_FROM_AMOUNT_START     = 261;
uint constant CTX0_FROM_AMOUNT_END       = 293;

uint constant CTX0_FROM_ASSET_LENGTH_POS = 293; 
uint constant CTX0_FROM_ASSET_START      = 294; 
uint constant CTX0_FROM_ASSET_START_EVM  = 338;  // If the address is an EVM address, this is the start
uint constant CTX0_FROM_ASSET_END        = 358;

uint constant CTX0_BLOCK_NUMBER_START    = 358;
uint constant CTX0_BLOCK_NUMBER_END      = 362;

uint constant CTX0_DATA_LENGTH_START     = 362;
uint constant CTX0_DATA_LENGTH_END       = 364;

uint constant CTX0_DATA_START            = 364;



// CTX1 Liquidity Swap Payload **************************************************************************************************

uint constant CTX1_MIN_VAULT_TOKEN_START = 228;
uint constant CTX1_MIN_VAULT_TOKEN_END   = 260;

uint constant CTX1_MIN_REFERENCE_START   = 260;
uint constant CTX1_MIN_REFERENCE_END     = 292;

uint constant CTX1_FROM_AMOUNT_START     = 292;
uint constant CTX1_FROM_AMOUNT_END       = 324;

uint constant CTX1_BLOCK_NUMBER_START    = 324;
uint constant CTX1_BLOCK_NUMBER_END      = 328;

uint constant CTX1_DATA_LENGTH_START     = 328;
uint constant CTX1_DATA_LENGTH_END       = 330;

uint constant CTX1_DATA_START            = 330;



// CTX2 Please Underwrite Payload ******************************************************************************************************

uint constant CTX2_TO_ASSET_INDEX_POS    = 228;

uint constant CTX2_MIN_OUT_START         = 229;
uint constant CTX2_MIN_OUT_END           = 261;

uint constant CTX2_FROM_AMOUNT_START     = 261;
uint constant CTX2_FROM_AMOUNT_END       = 293;

uint constant CTX2_FROM_ASSET_LENGTH_POS = 293; 
uint constant CTX2_FROM_ASSET_START      = 294; 
uint constant CTX2_FROM_ASSET_START_EVM  = 338;  // If the address is an EVM address, this is the start
uint constant CTX2_FROM_ASSET_END        = 358;

uint constant CTX2_BLOCK_NUMBER_START    = 358;
uint constant CTX2_BLOCK_NUMBER_END      = 362;

uint constant CTX2_TO_ACCOUNT_FALLBACK_LENGTH_POS   = 362;
uint constant CTX2_TO_ACCOUNT_FALLBACK_START        = 363;
uint constant CTX2_TO_ACCOUNT_FALLBACK_START_EVM    = 407;  // If the address is an EVM address, this is the start
uint constant CTX2_TO_ACCOUNT_FALLBACK_END          = 427;

uint constant CTX2_UW_INCENTIVE_START    = 427;
uint constant CTX2_UW_INCENTIVE_END      = 429;

uint constant CTX2_DATA_LENGTH_START     = 429;
uint constant CTX2_DATA_LENGTH_END       = 431;

uint constant CTX2_DATA_START            = 431;



// CTX3 Purpose Underwrite Payload ******************************************************************************************************

uint constant CTX3_TO_ASSET_INDEX_POS    = 228;

uint constant CTX3_FROM_AMOUNT_START     = 229;
uint constant CTX3_FROM_AMOUNT_END       = 261;

uint constant CTX3_FROM_ASSET_LENGTH_POS = 261; 
uint constant CTX3_FROM_ASSET_START      = 262; 
uint constant CTX3_FROM_ASSET_START_EVM  = 306;  // If the address is an EVM address, this is the start
uint constant CTX3_FROM_ASSET_END        = 326;

uint constant CTX3_BLOCK_NUMBER_START    = 326;
uint constant CTX3_BLOCK_NUMBER_END      = 330;

uint constant CTX3_UNDERWRITE_ID_START   = 330;
uint constant CTX3_UNDERWRITE_ID_END     = 362;

uint constant CTX3_DATA_LENGTH_START     = 362;
uint constant CTX3_DATA_LENGTH_END       = 364;

uint constant CTX3_DATA_START            = 384;