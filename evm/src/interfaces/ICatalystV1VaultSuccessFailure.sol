//SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

/// @title Escrow related functions defined by Catalyst v1 Vaults
/// @notice Contains the functions used to manage escrows by the cross-chain interface.
interface ICatalystV1VaultSuccessFailure {
    /** @notice Release the escrowed tokens into the vault.  */
    function onSendAssetSuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) external;

    /** @notice Returned the escrowed tokens to the user */
    function onSendAssetFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) external;

    /** @notice Release the escrowed tokens into the vault.  */
    function onSendLiquiditySuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) external;

    /** @notice Returned the escrowed tokens to the user */
    function onSendLiquidityFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) external;
}
