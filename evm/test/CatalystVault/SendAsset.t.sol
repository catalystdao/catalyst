// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "../../src/ICatalystV1Vault.sol";
import {Token} from "../mocks/token.sol";
import "../../src/utils/FixedPointMathLib.sol";

interface TF {
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool);
}

abstract contract TestSendAsset is TestCommon {

    uint256 constant MARGIN_NUM = 1;
    uint256 constant MARGIN_DENOM = 1e18;

    function invariant(address[] memory vaults) view virtual internal returns(uint256 inv);

    function getTestConfig() virtual internal returns(address[] memory vaults);

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset) virtual internal returns(uint256 amount);
    
    function test_sendAsset(bytes32 channelId, uint56 amount, address toAccount) external virtual {
        vm.assume(toAccount != address(0));
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;
        uint8 fromAssetIndex = 0;
        uint8 toAssetIndex = 0;

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];

        setConnection(fromVault, toVault, channelId, channelId);

        uint256 initial_invariant = invariant(vaults);

        address fromToken = ICatalystV1Vault(fromVault)._tokenIndexing(fromAssetIndex);
        Token(fromToken).approve(fromVault, amount);

        uint256 units = ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            channelId,
            convertEVMTo65(toVault),
            convertEVMTo65(toAccount),
            fromToken,
            toAssetIndex, 
            uint256(amount), 
            uint256(amount)/2, 
            toAccount, 
            _INCENTIVE
        );

        uint256 after_invariant = invariant(vaults);

        if (after_invariant + units < initial_invariant) {
            assertGt(
                initial_invariant * MARGIN_NUM / MARGIN_DENOM,
                initial_invariant - after_invariant,
                "Swap error beyond margin found"
            );
        }
    }
}

