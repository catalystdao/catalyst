// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../../src/ICatalystV1Vault.sol";
import {Token} from "../mocks/token.sol";
import "../../src/utils/FixedPointMathLib.sol";

abstract contract TestLocalswap is Test {

    uint256 constant MARGIN_NUM = 1;
    uint256 constant MARGIN_DENOM = 1e18;

    function invariant(address[] memory vaults) virtual internal returns(uint256 inv);

    function getTestConfig() virtual internal returns(address[] memory vaults);

    function t_only_localswap(address vault, uint256 amount, address fromAsset, address toAsset) internal {
        ICatalystV1Vault v = ICatalystV1Vault(vault);

        Token(fromAsset).approve(vault, amount);

        v.localSwap(fromAsset, toAsset, amount, 0);
    }

    function t_localswap(address[] memory vaults, address swapVault, uint256 amount, address fromAsset, address toAsset) internal {
        uint256 initial_invariant = invariant(vaults);

        t_only_localswap(swapVault, amount, fromAsset, toAsset);

        uint256 after_invariant = invariant(vaults);

        // We allow upto a (very small)% decrease. If the pool size is $1 million million million ($1e18), then we are okay with losing 1$.
        if (after_invariant < initial_invariant) {
            assertGt(
                initial_invariant * MARGIN_NUM / MARGIN_DENOM,
                initial_invariant - after_invariant,
                "Swap error beyond margin found"
            );
        }
    }

    function test_local_swap_invariance(uint32 percentageOfPool) external virtual {
        address[] memory vaults = getTestConfig();

        address swapVault = vaults[0];

        address fromToken = ICatalystV1Vault(swapVault)._tokenIndexing(0);
        address toToken = ICatalystV1Vault(swapVault)._tokenIndexing(1);

        uint256 swapAmount = Token(fromToken).balanceOf(swapVault) * uint256(percentageOfPool) / (2**32 - 1);

        t_localswap(vaults, swapVault, swapAmount, fromToken, toToken);
    }
}

