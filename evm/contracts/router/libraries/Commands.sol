// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

/// @title Commands
/// @notice Command Flags used to decode commands
library Commands {
    // Masks to extract certain bits of commands
    bytes1 internal constant FLAG_ALLOW_REVERT = 0x80;
    bytes1 internal constant COMMAND_TYPE_MASK = 0x3f;

    // Command Types. Maximum supported command at this moment is 0x1F.

    // Block 1
    uint256 constant LOCALSWAP              = 0x00;
    uint256 constant SENDASSET              = 0x01;
    uint256 constant PERMIT2_TRANSFER_FROM  = 0x02;
    uint256 constant PERMIT2_PERMIT_BATCH   = 0x03;
    uint256 constant SWEEP                  = 0x04;
    uint256 constant TRANSFER               = 0x05;
    uint256 constant PAY_PORTION            = 0x06;

    // Block 2
    uint256 constant PERMIT2_PERMIT         = 0x07;
    uint256 constant WRAP_GAS               = 0x08;
    uint256 constant UNWRAP_GAS             = 0x09;
    uint256 constant WITHDRAW_EQUAL         = 0x0a;
    uint256 constant WITHDRAW_MIXED         = 0x0b;
    uint256 constant DEPOSIT_MIXED          = 0x0c;
    uint256 constant ALLOW_CANCEL           = 0x0d;
    uint256 constant BALANCE_CHECK_ERC20    = 0x0e;

    // Command Types where 0x10<=value
    uint256 constant EXECUTE_SUB_PLAN       = 0x10;
}