// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solmate/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestWithdrawInvariant is TestCommon, AVaultInterfaces {
    /// todo: fix
    function disabled_invariant_withdraw_mixed(uint32 withdrawPercentage) external {
        vm.assume(withdrawPercentage != type(uint32).max);
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            // Get number of tokens:
            uint256 numTokens = 0;
            while (true) {
                if (v._tokenIndexing(numTokens) == address(0)) {
                    break;
                }
                ++numTokens;
            }

            // If withdraw percentage is below 0.1%, then make it 0.1%.
            if (withdrawPercentage < 2**32/2**22) {
                withdrawPercentage = 2**32/2**22;
            }
            uint256 amountToWithdraw = Token(address(v)).balanceOf(address(this)) * uint256(withdrawPercentage) / (2**32 - 1);

            // Make minout array
            uint256[] memory minOut = new uint256[](numTokens);


            // Withdraw ratios using progressive weights:
            uint256[] memory weights = new uint256[](numTokens);
            for (uint256 j = 0; j < numTokens; ++j) {
                weights[j] = 10**18 / (numTokens - j);
            }

            // Invariant before withdrawal
            uint256 invariantBefore = strong_invariant(vault);

            // WithdrawMixed
            v.withdrawMixed(amountToWithdraw, weights, minOut);

            // Invariant after withdrawal
            uint256 invariantAfter = strong_invariant(vault);

            // Check that it didn't decrease:
            assertGt(invariantAfter * 105/100, invariantBefore, "Invariant decreased after withdrawal");
        }
    }
}