// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestWithdrawComparison is TestCommon, AVaultInterfaces {
    /// @notice Compare the output difference between withdrawAll and withdrawMixed.
    function test_compare_withdraw(uint32 withdrawPercentage) external {
        vm.assume(withdrawPercentage != type(uint32).max);
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            // Get number of tokens:
            uint256 numTokens = 0; // TODO: balances.length;
            // If withdraw percentage is below 0.1%, then make it 0.1%.
            if (withdrawPercentage < 2**32/2**22) {
                withdrawPercentage = 2**32/2**22;
            }
            uint256 amountToWithdraw = Token(address(v)).balanceOf(address(this)) * uint256(withdrawPercentage) / (2**32 - 1);

            // Take a snapshot:
            uint256 snapshotId = vm.snapshot();

            // Make minout array
            uint256[] memory minOut = new uint256[](numTokens);

            // WithdrawEqual
            uint256[] memory outsAll = v.withdrawAll(amountToWithdraw, minOut);

            vm.revertTo(snapshotId);

            // Withdraw ratios using progressive weights:
            uint256[] memory weights = new uint256[](numTokens);
            for (uint256 j = 0; j < numTokens; ++j) {
                weights[j] = 10**18 / (numTokens - j);
            }

            // WithdrawMixed
            uint256[] memory outsMixed = v.withdrawMixed(amountToWithdraw, weights, minOut);

            // Check that outsAll and outsMixed are equal
            for (uint256 j = 0; j < numTokens; ++j) {
                assertEq(outsAll[j], outsMixed[j], "Not equal withdrawn amoutns");
            }
        }
    }
}