//SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.16;

/// @title Immutable vault state
/// @notice Contains all vault state which doesn't change once set.
interface ICatalystV1VaultImmutables {
    function _chainInterface() external view returns (address);

    function FACTORY() external view returns (address);

    function MATHLIB() external view returns (address);

    /// @notice To indicate which token is desired on the target vault, the _toAsset is an integer from 0 to MAX_ASSETS indicating which asset the vault should purchase with units.
    function _tokenIndexing(uint256 tokenIndex) external view returns (address);
}
