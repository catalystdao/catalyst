//SPDX-License-Identifier: Unlicsened
pragma solidity ^0.8.17;

/// @title Derived Pool state
/// @notice Contains all pool state which is derived from pool storage
interface ICatalystV1PoolDerived {
    /** @notice  Returns the current cross-chain unit capacity. */
    function getUnitCapacity() external view returns (uint256);
}
