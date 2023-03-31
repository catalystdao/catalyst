//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

// Catalyst IBC payload structure ***********************************************************************************************
//
// Common Payload (beginning)
//    CONTEXT               0   (1 byte)   
//    + FROM_POOL           1   (64 bytes) 
//    + TO_POOL             65  (64 bytes)
//    + TO_ACCOUNT          129  (64 bytes) 
//    + UNITS               193  (32 bytes) 
// 
// Context-depending Payload
//    CTX0 - 0x00 - Asset Swap Payload
//       + TO_ASSET_INDEX   225 (1 byte)
//       + MIN_OUT          226 (32 bytes)
//       + FROM_AMOUNT      258 (32 bytes)
//       + FROM_ASSET       290 (64 bytes)
//       + BLOCK_NUMBER     354 (4 bytes)
//       + SWAP_HASH        358 (32 bytes)
//
//    CTX1 - 0x01 - Liquidity Swap Payload
//       + MIN_OUT          225 (32 bytes)
//       + FROM_AMOUNT      257 (32 bytes)
//       + BLOCK_NUMBER     289 (4 bytes)
//       + SWAP_HASH        293 (32 bytes)
//
// Common Payload (end)
//    + DATA_LENGTH         LENGTH-N-2 (2 bytes)
//    + DATA                LENGTH-N   (N bytes)



// Contexts *********************************************************************************************************************

bytes1 constant CTX0_ASSET_SWAP     = 0x00;
bytes1 constant CTX1_LIQUIDITY_SWAP = 0x01;



// Common Payload ***************************************************************************************************************

uint constant CONTEXT_POS           = 0;

uint constant FROM_POOL_START       = 1;
uint constant FROM_POOL_START_EVM   = 45;  // If the address is an EVM address, this is the start
uint constant FROM_POOL_END         = 65;

uint constant TO_POOL_START         = 65;
uint constant TO_POOL_START_EVM     = 109;  // If the address is an EVM address, this is the start
uint constant TO_POOL_END           = 129;

uint constant TO_ACCOUNT_START      = 129;
uint constant TO_ACCOUNT_START_EVM  = 173;  // If the address is an EVM address, this is the start
uint constant TO_ACCOUNT_END        = 193;

uint constant UNITS_START           = 193;
uint constant UNITS_END             = 225;



// CTX0 Asset Swap Payload ******************************************************************************************************

uint constant CTX0_TO_ASSET_INDEX_POS    = 225;

uint constant CTX0_MIN_OUT_START         = 226;
uint constant CTX0_MIN_OUT_END           = 258;

uint constant CTX0_FROM_AMOUNT_START     = 258;
uint constant CTX0_FROM_AMOUNT_END       = 290;

uint constant CTX0_FROM_ASSET_START      = 290; 
uint constant CTX0_FROM_ASSET_START_EVM  = 334;  // If the address is an EVM address, this is the start
uint constant CTX0_FROM_ASSET_END        = 354;

uint constant CTX0_BLOCK_NUMBER_START    = 354;
uint constant CTX0_BLOCK_NUMBER_END      = 358;

uint constant CTX0_SWAP_HASH_START       = 358;
uint constant CTX0_SWAP_HASH_END         = 390;

uint constant CTX0_DATA_LENGTH_START     = 390;
uint constant CTX0_DATA_LENGTH_END       = 392;

uint constant CTX0_DATA_START            = 392;



// CTX1 Liquidity Swap Payload **************************************************************************************************

uint constant CTX1_MIN_OUT_START         = 225;
uint constant CTX1_MIN_OUT_END           = 257;

uint constant CTX1_FROM_AMOUNT_START     = 257;
uint constant CTX1_FROM_AMOUNT_END       = 289;

uint constant CTX1_BLOCK_NUMBER_START    = 289;
uint constant CTX1_BLOCK_NUMBER_END      = 293;

uint constant CTX1_SWAP_HASH_START       = 293;
uint constant CTX1_SWAP_HASH_END         = 325;

uint constant CTX1_DATA_LENGTH_START     = 325;
uint constant CTX1_DATA_LENGTH_END       = 327;

uint constant CTX1_DATA_START            = 327;