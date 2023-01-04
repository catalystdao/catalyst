//SPDX-License-Identifier: Unlicsened
pragma solidity ^0.8.16;

/// @title Errors defined by Catalyst v1 Pools
/// @notice Contains all errors raised by the pool
contract ICatalystV1PoolErrors {
    string constant EXCEEDS_SECURITY_LIMIT =
        "Swap exceeds maximum swap amount. Please wait";
    string constant SWAP_RETURN_INSUFFICIENT = "Insufficient Return";
    string constant BALANCE_SECURITY_LIMIT =
        "Pool Sanity Limit (Balance too large)";
}
