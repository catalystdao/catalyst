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
        bytes32 identifier,
        uint256 escrowAmount,
        address escrowToken
    ) external;

    function deleteUnderwriteAsset(
        bytes32 identifier,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external;
}