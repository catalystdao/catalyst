//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

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

bytes1 constant CTX0_ASSET_SWAP         = 0x00;
bytes1 constant CTX1_LIQUIDITY_SWAP     = 0x01;



// Common Payload ***************************************************************************************************************

uint constant CONTEXT                   = 1;

uint constant FROM_POOL_LENGTH          = 1;

uint constant TO_POOL_LENGTH            = 1;

uint constant TO_ACCOUNT_LENGTH         = 1;

uint constant UNIT_BYTES                = 32;



// CTX0 Asset Swap Payload ******************************************************************************************************

uint constant CTX0_TO_ASSET_INDEX       = 1;

uint constant CTX0_MIN_OUT              = 32;

uint constant CTX0_FROM_AMOUNT          = 32;

uint constant CTX0_FROM_ASSET_LENGTH    = 1; 

uint constant CTX0_BLOCK_NUMBER         = 4;

uint constant CTX0_SWAP_HASH            = 32;

uint constant CTX0_DATA_LENGTH          = 2;



// CTX1 Liquidity Swap Payload **************************************************************************************************

uint constant CTX1_MIN_OUT              = 32;

uint constant CTX1_FROM_AMOUNT          = 32;

uint constant CTX1_BLOCK_NUMBER         = 4;

uint constant CTX1_SWAP_HASH            = 32;

uint constant CTX1_DATA_LENGTH          = 2;