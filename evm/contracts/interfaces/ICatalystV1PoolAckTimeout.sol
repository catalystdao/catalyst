//SPDX-License-Identifier: Unlicsened
pragma solidity >=0.8.17 <0.9.0;

/// @title Escrow related functions defined by Catalyst v1 Pools
/// @notice Contains the functions used to manage escrows by the cross-chain interface.
interface ICatalystV1PoolAckTimeout {
    /** @notice Release the escrowed tokens into the pool.  */
    function releaseEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external;

    /** @notice Returned the escrowed tokens to the user */
    function releaseEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external;

    /** @notice Release the escrowed tokens into the pool.  */
    function releaseLiquidityEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external;

    /** @notice Returned the escrowed tokens to the user */
    function releaseLiquidityEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external;
}
