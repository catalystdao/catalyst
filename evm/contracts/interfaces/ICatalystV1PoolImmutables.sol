//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Immutable pool state
/// @notice Contains all pool state which doesn't change once set.
interface ICatalystV1PoolImmutables {
    function _chaininterface() external view returns (address);

    function FACTORY() external view returns (address);

    /// @notice
    ///     To indicate which token is desired on the target pool,
    ///     the _toAsset is an integer from 0 to NUMASSETS indicating
    ///     which asset the pool should purchase with units.
    function _tokenIndexing(uint256 tokenIndex) external view returns (address);
}
