//SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

/// @title Derived Vault state
/// @notice Contains all vault state which is derived from vault storage
interface ICatalystV1VaultDerived {
    /** @notice  Returns the current cross-chain unit capacity. */
    function getUnitCapacity() external view returns (uint256);

    function calcSendAsset(address from, uint256 amount) external view returns (uint256);

    /**
     * @notice Computes the output of ReceiveAsset, without executing one.
     * @param to The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveAsset(address to, uint256 U) external view returns (uint256);

    /**
     * @notice Computes the output of localSwap, without executing one.
     * @param from The address of the token to sell.
     * @param to The address of the token to buy.
     * @param amount The amount of from token to sell for to token.
     * @return Output denominated in to token.
     */
    function calcLocalSwap(address from, address to, uint256 amount) external view returns (uint256);
}
