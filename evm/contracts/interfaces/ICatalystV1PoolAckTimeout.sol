//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Escrow related functions defined by Catalyst v1 Pools
/// @notice Contains the functions used to manage escrows by the cross-chain interface.
interface ICatalystV1PoolAckTimeout {
    /** @notice Release the escrowed tokens into the pool.  */
    function sendAssetAck(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) external;

    /** @notice Returned the escrowed tokens to the user */
    function sendAssetTimeout(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) external;

    /** @notice Release the escrowed tokens into the pool.  */
    function sendLiquidityAck(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) external;

    /** @notice Returned the escrowed tokens to the user */
    function sendLiquidityTimeout(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) external;
}
