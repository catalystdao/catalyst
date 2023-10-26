// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestCompareDepositWithWithdraw is TestCommon, AVaultInterfaces {
    /// @notice Compare the output difference between withdrawAll and withdrawMixed.
    function test_compare_deposit_withdraw(uint32 depositPercentage) external {
        vm.assume(depositPercentage != type(uint32).max);
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);
            // Get number of tokens:
            uint256 numTokens = 0;
            for (numTokens = 0; numTokens < 100; ++numTokens) {
                address tkn = v._tokenIndexing(numTokens);
                if (v._tokenIndexing(numTokens) == address(0)) {
                    break;
                }
            }

            uint256[] memory depositAmounts = new uint256[](numTokens);
            for (uint256 i = 0; i < numTokens; ++i) {
                address tkn = v._tokenIndexing(i);
                depositAmounts[i] = Token(tkn).balanceOf(address(this)) * uint256(depositPercentage) / (2**32 - 1);
                Token(tkn).approve(vault, depositAmounts[i]);
            }

            // Get invariant
            uint256 inv = invariant(vaults);

            // Deposit
            uint256 poolTokensMinted = v.depositMixed(depositAmounts, 0);

            uint256[] memory minOut = new uint256[](numTokens);
            // Withdraw using withdraw all
            uint256[] memory outsAll = v.withdrawAll(poolTokensMinted, minOut);

            uint256 invAfter = invariant(vaults);

            // Check that the invariant didn't decrease. A small buffer is added because the math isn't exact.
            assertLt(inv, invAfter * (10 ** 18 + 10 ** 6) / 10 ** 18, "Invariant decreased");
        }
    }
}