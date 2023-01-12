//SPDX-License-Identifier: Unlicsened
pragma solidity ^0.8.16;

/// @title Errors defined by Catalyst v1 Pools
/// @notice Contains all errors raised by the pool
contract ICatalystV1PoolErrors {
    string constant EXCEEDS_SECURITY_LIMIT =
        "Swap exceeds security limit";
    string constant SWAP_RETURN_INSUFFICIENT = "Insufficient Return";
}
