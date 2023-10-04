//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.17;

/// @title Extensions to vaults which supports underwriting.
interface ICatalystV1Underwriting {
    function underwriteAsset(
        bytes32 identifier,
        address toAsset,
        uint256 U,
        uint256 minOut
    ) external returns (uint256);

    function releaseUnderwriteAsset(
        address refundTo,
        bytes32 identifier,
        uint256 escrowAmount,
        address escrowToken,
        bytes32 sourceIdentifier,
        bytes calldata fromVault
    ) external;

    function deleteUnderwriteAsset(
        bytes32 identifier,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external;
}