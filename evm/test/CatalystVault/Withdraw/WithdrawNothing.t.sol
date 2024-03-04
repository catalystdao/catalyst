// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import { ICatalystV1Vault } from "src/ICatalystV1Vault.sol";
import { FixedPointMathLib as Math } from "solady/utils/FixedPointMathLib.sol";

import "forge-std/Test.sol";
import { TestCommon } from "test/TestCommon.t.sol";
import { Token } from "test/mocks/token.sol";
import { AVaultInterfaces } from "test/CatalystVault/AVaultInterfaces.t.sol";
import { TestInvariant } from "test/CatalystVault/Invariant.t.sol";


function queryAssetCount(ICatalystV1Vault vault) view returns (uint256) {
    uint256 tokenCount = 0;
    for (uint256 i; true; i++) {
        address token = vault._tokenIndexing(i);
        if (token == address(0)) return tokenCount;
        else tokenCount += 1;
    }
}

function queryWeightsSum(ICatalystV1Vault vault) view returns (uint256) {
    uint256 weightsSum = 0;
    for (uint256 i; true; i++) {
        uint256 weight = vault._weight(vault._tokenIndexing(i));
        if (weight == 0) return weightsSum;
        else weightsSum += weight;
    }
}

function getEvenWithdrawRatios(uint256 assetCount) returns (uint256[] memory) {

    uint256[] memory ratios = new uint256[](assetCount);

    for (uint256 i; i < assetCount; i++) {
        ratios[i] = Math.WAD / (assetCount - i);
    }

    return ratios;
}


abstract contract TestWithdrawNothing is TestCommon, AVaultInterfaces {


    function test_WithdrawNothing() external {

        // Withdrawing 0 vault tokens yields nothing/reverts

        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {

            ICatalystV1Vault vault = ICatalystV1Vault(vaults[i]);
            uint256 assetCount = queryAssetCount(vault);

            address withdrawer = address(1);
            assert(Token(address(vault)).balanceOf(withdrawer) == 0);

            uint256 snapshot = vm.snapshot();



            // Tested action 1: withdraw all
            vm.prank(withdrawer);
            vault.withdrawAll(0, new uint256[](assetCount));

            // Verify no assets have been received
            for (uint j = 0; true; j++) {
                address token = vault._tokenIndexing(j);

                if (token != address(0)) {
                    assert(Token(token).balanceOf(withdrawer) == 0);
                }
                else break;
            }



            // Tested action 2: withdraw mixed
            vm.revertTo(snapshot);
            vm.prank(withdrawer);
            vm.expectRevert();  // Reverts as `withdrawMixed` is not compatible with withdrawing 0 vault tokens.
            vault.withdrawMixed(0, getEvenWithdrawRatios(assetCount), new uint256[](assetCount));

        }

    }


    function test_WithdrawOne() external {

        // Withdrawing 1 vault token is correct/may revert

        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {

            ICatalystV1Vault vault = ICatalystV1Vault(vaults[i]);
            uint256 assetCount = queryAssetCount(vault);
            uint256 totalSupply = Token(address(vault)).totalSupply();
            uint256 withdrawAmount = 1;

            address withdrawer = address(1);

            Token(address(vault)).transfer(withdrawer, withdrawAmount);

            uint256[] memory initialBalances = new uint256[](assetCount);
            for (uint j; j < assetCount; j++) {
                Token token = Token(vault._tokenIndexing(j));
                initialBalances[j] = token.balanceOf(address(vault));
            }

            uint256 snapshot = vm.snapshot();
            



            // Tested action 1: withdraw all
            vm.prank(withdrawer);
            uint256[] memory tokenOutputs = vault.withdrawAll(
                withdrawAmount,
                new uint256[](assetCount)  // Set minimum output to 0
            );

            // Check the withdrawn amounts are approx correct
            for (uint j; j < assetCount; j++) {
                assert(
                    tokenOutputs[j] <= initialBalances[j] * withdrawAmount / totalSupply * 101 / 100
                );
            }



            // Tested action 2: withdraw mixed
            vm.revertTo(snapshot);
            vm.prank(withdrawer);
            
            // `withdrawMixed` may revert for very small withdrawals.
            try vault.withdrawMixed(
                withdrawAmount,
                getEvenWithdrawRatios(assetCount),
                new uint256[](assetCount)  // Set minimum output to 0
            ) returns (uint256[] memory callTokenOutputs) {
                 
                // If the transaction does not revert, verify that the token return was very small
                for (uint j; j < callTokenOutputs.length; j++) {
                    address token = vault._tokenIndexing(j);
                    if (token == address(0)) break;
                    assertLt(
                        callTokenOutputs[j] * 100000000 / Token(token).balanceOf(address(vault)),
                        1
                    );
                }
            }
            catch {

                // Check the withdrawn amounts are approx correct
                for (uint j; j < assetCount; j++) {
                    if (!amplified) continue;   // TODO implement once the `withdrawMixed` ratios implementation is overhauled 
                    assert(
                        tokenOutputs[j] <= initialBalances[j] * withdrawAmount / totalSupply * 101 / 100
                    );
                }
            }
        }

    }


    function test_WithdrawVerySmall() external {

        // Small withdrawals work

        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {

            ICatalystV1Vault vault = ICatalystV1Vault(vaults[i]);

            uint256 assetCount = queryAssetCount(vault);
            uint256 totalSupply = Token(address(vault)).totalSupply();
            uint256 withdrawAmount = 1000000;

            address withdrawer = address(1);

            Token(address(vault)).transfer(withdrawer, withdrawAmount);


            uint256[] memory initialBalances = new uint256[](assetCount);
            for (uint j; j < assetCount; j++) {
                Token token = Token(vault._tokenIndexing(j));
                initialBalances[j] = token.balanceOf(address(vault));
            }

            uint256 snapshot = vm.snapshot();



            // Tested action 1: withdraw all
            vm.prank(withdrawer);
            uint256[] memory tokenOutputs = vault.withdrawAll(
                withdrawAmount,
                new uint256[](assetCount)  // Set minimum output to 0
            );


            // Check the withdrawn amounts are approx correct
            for (uint j; j < assetCount; j++) {
                assert(
                    tokenOutputs[j] <= initialBalances[j] * withdrawAmount / totalSupply * 101 / 100
                );
            }



            // Tested action 2: withdraw mixed
            vm.revertTo(snapshot);
            vm.prank(withdrawer);
            tokenOutputs = vault.withdrawMixed(
                withdrawAmount,
                getEvenWithdrawRatios(assetCount),
                new uint256[](assetCount)  // Set minimum output to 0
            );

            // Check the withdrawn amounts are approx correct
            for (uint j; j < assetCount; j++) {
                if (!amplified) continue;   // TODO implement once the `withdrawMixed` ratios implementation is overhauled 
                assert(
                    tokenOutputs[j] <= initialBalances[j] * withdrawAmount / totalSupply * 101 / 100
                );
            }
        }

    }

}