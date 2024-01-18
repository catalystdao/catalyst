// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import { ICatalystV1Vault } from "src/ICatalystV1Vault.sol";
import { FixedPointMathLib as Math } from "solmate/utils/FixedPointMathLib.sol";
import { CatalystVaultAmplified } from "src/CatalystVaultAmplified.sol";
import { WADWAD } from "src/utils/MathConstants.sol";

import "forge-std/Test.sol";
import { TestCommon } from "test/TestCommon.t.sol";
import { Token } from "test/mocks/token.sol";
import { AVaultInterfaces } from "test/CatalystVault/AVaultInterfaces.t.sol";
import { TestInvariant } from "test/CatalystVault/Invariant.t.sol";


function queryAssetCount(ICatalystV1Vault vault) returns (uint256) {
    uint256 tokenCount = 0;
    for (uint256 i; true; i++) {
        address token = vault._tokenIndexing(i);
        if (token == address(0)) return tokenCount;
        else tokenCount += 1;
    }
}

function queryWeightsSum(ICatalystV1Vault vault) returns (uint256) {
    uint256 weightsSum = 0;
    for (uint256 i; true; i++) {
        uint256 weight = vault._weight(vault._tokenIndexing(i));
        if (weight == 0) return weightsSum;
        else weightsSum += weight;
    }
}

function queryVaultBalances(ICatalystV1Vault vault) returns (uint256[] memory) {

    uint256 assetCount = queryAssetCount(vault);

    uint256[] memory balances = new uint256[](assetCount);
    for (uint i; i < assetCount; i++) {
        Token token = Token(vault._tokenIndexing(i));
        balances[i] = token.balanceOf(address(vault));
    }

    return balances;
}

function queryVaultWeights(ICatalystV1Vault vault) returns (uint256[] memory) {

    uint256 assetCount = queryAssetCount(vault);

    uint256[] memory weights = new uint256[](assetCount);
    for (uint i; i < assetCount; i++) {
        address token = vault._tokenIndexing(i);
        weights[i] = vault._weight(token);
    }

    return weights;
}


abstract contract TestWithdrawUnbalanced is TestCommon, AVaultInterfaces {


    // Helpers
    // ********************************************************************************************

    function calculateExpectedEqualWithdrawal(
        ICatalystV1Vault vault,
        uint256 withdrawAmount
    ) private returns (uint256[] memory) {
        
        uint256 totalSupply = Token(address(vault)).totalSupply();
        uint256[] memory vaultBalances = queryVaultBalances(vault);
        uint256 assetCount = vaultBalances.length;

        uint256[] memory expectedWithdrawAmounts = new uint256[](assetCount);

        if (!amplified) {
            for (uint256 i; i < assetCount; i++) {
                expectedWithdrawAmounts[i] = vaultBalances[i] * withdrawAmount / totalSupply;
            }
        }
        else {
            CatalystVaultAmplified ampVault = CatalystVaultAmplified(address(vault));

            uint256[] memory vaultWeights = queryVaultWeights(vault);

            int256 oneMinusAmp = ampVault._oneMinusAmp();
            int256 balance0 = int256(ampVault.computeBalance0());

            int256 inner = totalSupply == withdrawAmount ? int256(Math.WAD) : int256(Math.WAD) - Math.powWad(
                int256(Math.WAD * (totalSupply - withdrawAmount) / totalSupply),
                oneMinusAmp
            );
            inner = inner * Math.powWad(balance0, oneMinusAmp) / int256(Math.WAD);

            for (uint256 i; i < assetCount; i++) {
                int256 weightedBalance = int256(vaultBalances[i] * vaultWeights[i] * Math.WAD);

                if (Math.powWad(weightedBalance, oneMinusAmp) >= inner) {
                    int256 weightedBalanceAmped = Math.powWad(weightedBalance, oneMinusAmp);
                    expectedWithdrawAmounts[i] = uint256(
                        weightedBalance - (weightedBalanceAmped == inner ? int256(0) : Math.powWad(
                            weightedBalanceAmped - inner,
                            WADWAD / oneMinusAmp
                        ))
                    )  / vaultWeights[i] / Math.WAD;
                }
                else {
                    expectedWithdrawAmounts[i] = vaultBalances[i];
                }
            }
        }

        return expectedWithdrawAmounts;
    }



    // Tests
    // ********************************************************************************************

    function test_WithdrawAllUnbalanced(
        uint32 unbalancePercentage,
        uint32 withdrawalPercentage
    ) external {

        vm.assume(unbalancePercentage >= type(uint32).max/100);
        vm.assume(withdrawalPercentage >= type(uint32).max/100);


        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {

            ICatalystV1Vault vault = ICatalystV1Vault(vaults[i]);
            uint256 assetCount = queryAssetCount(vault);
            uint256[] memory initialBalances = queryVaultBalances(vault);
            uint256 totalSupply = Token(address(vault)).totalSupply();

            address user = address(1);


            // Perform a swap to 'unbalance' the vault
            uint256 swapAmount = initialBalances[0] * unbalancePercentage / type(uint32).max;

            Token fromToken = Token(vault._tokenIndexing(0));
            Token toToken = Token(vault._tokenIndexing(1));
            fromToken.transfer(user, swapAmount);

            vm.prank(user);
            fromToken.approve(address(vault), swapAmount);

            vm.prank(user);
            vault.localSwap(
                address(fromToken),
                address(toToken),
                swapAmount,
                0
            );

            uint256[] memory afterSwapBalances = queryVaultBalances(vault);


            // Execute the withdrawal
            uint256 withdrawAmount = totalSupply * withdrawalPercentage / type(uint32).max;

            uint256[] memory expectedAmounts = calculateExpectedEqualWithdrawal(vault, withdrawAmount);

            Token(address(vault)).transfer(user, withdrawAmount);
            vm.prank(user);
            uint256[] memory withdrawOutput = vault.withdrawAll(
                withdrawAmount,
                new uint256[](assetCount)  // Set minimum output to 0
            );


            // Check the withdrawal return
            for (uint256 j; j < assetCount; j++) {
                uint256 expectedAmount = expectedAmounts[j];
                uint256 withdrawnAmount = withdrawOutput[j];

                assert(
                    withdrawnAmount <= expectedAmount * 10000000001 / 10000000000 + 1
                );
                assert(
                    withdrawnAmount >= expectedAmount * 99 / 100
                );
            }
        }

    }


    function test_CompareWithdrawAllAndMixedUnbalanced(uint32 unbalancePercentage,uint32 withdrawalPercentage) external {
        vm.assume(unbalancePercentage >= type(uint32).max/100);
        vm.assume(withdrawalPercentage >= type(uint32).max/100);


        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {

            ICatalystV1Vault vault = ICatalystV1Vault(vaults[i]);
            uint256 assetCount = queryAssetCount(vault);
            uint256[] memory initialBalances = queryVaultBalances(vault);
            uint256 totalSupply = Token(address(vault)).totalSupply();

            address user = address(1);


            // Perform a swap to 'unbalance' the vault
            uint256 swapAmount = initialBalances[0] * unbalancePercentage / type(uint32).max;

            Token fromToken = Token(vault._tokenIndexing(0));
            Token toToken = Token(vault._tokenIndexing(1));
            fromToken.transfer(user, swapAmount);

            vm.prank(user);
            fromToken.approve(address(vault), swapAmount);

            vm.prank(user);
            vault.localSwap(
                address(fromToken),
                address(toToken),
                swapAmount,
                0
            );

            uint256[] memory afterSwapBalances = queryVaultBalances(vault);

            uint256 withdrawAmount = totalSupply * withdrawalPercentage / type(uint32).max;
            Token(address(vault)).transfer(user, withdrawAmount);

            uint256 beforeWithdrawalSnapshot = vm.snapshot();


            // Execute `WithdrawAll`
            vm.prank(user);
            uint256[] memory withdrawAllOutput = vault.withdrawAll(
                withdrawAmount,
                new uint256[](assetCount)  // Set minimum output to 0
            );

            uint256[] memory withdrawalWeights = new uint256[](assetCount);
            for (uint256 j = 0; j < assetCount; ++j) {
                withdrawalWeights[j] = 10**18;
            }
            // Execute `WithdrawMixed`
            vm.revertTo(beforeWithdrawalSnapshot);
            vm.prank(user);
            uint256[] memory withdrawMixedOutput;
            try vault.withdrawMixed(
                withdrawAmount,
                getWithdrawPercentages(address(vault), withdrawalWeights),
                new uint256[](assetCount)  // Set minimum output to 0
            ) returns (uint256[] memory _withdrawMixedOutput) {
                withdrawMixedOutput = _withdrawMixedOutput;
            }
            catch {
                // Skip comparison and continue
                // TODO do a more exhaustive check here?
                continue;
            }

            for (uint256 j; j < assetCount; j++) {
                uint256 withdrawAllAmount = withdrawAllOutput[j];
                uint256 withdrawMixedAmount = withdrawMixedOutput[j];

                assert(
                    withdrawMixedAmount <= withdrawAllAmount * 10000000001 / 10000000000 + 1
                );
                assert(
                    withdrawMixedAmount >= withdrawAllAmount * 99 / 100
                );
            }

        }

    }

}