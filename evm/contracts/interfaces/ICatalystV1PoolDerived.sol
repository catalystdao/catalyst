//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Derived Pool state
/// @notice Contains all pool state which is derived from pool storage
interface ICatalystV1PoolDerived {
    /** @notice  Returns the current cross-chain unit capacity. */
    function getUnitCapacity() external view returns (uint256);

    function calcSendSwap(address from, uint256 amount) external view returns (uint256);

    /**
     * @notice Computes the output of SwapFromUnits, without executing one.
     * @param to The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveSwap(address to, uint256 U) external view returns (uint256);

    /**
     * @notice Computes the output of SwapToAndFromUnits, without executing one.
     * @param from The address of the token to sell.
     * @param to The address of the token to buy.
     * @param amount The amount of from token to sell for to token.
     * @return Output denominated in to token.
     */
    function calcLocalSwap(address from, address to, uint256 amount) external view returns (uint256);
}
