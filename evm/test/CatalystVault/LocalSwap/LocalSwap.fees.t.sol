// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestLocalswapFees is Test, AVaultInterfaces {
    function test_local_swap_with_fee(uint16 swapPercentage, uint48 vaultFee) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];


            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            address toToken = ICatalystV1Vault(vault)._tokenIndexing(1);

            uint256 swapAmount = getLargestSwap(vault, vault, fromToken, toToken) * swapPercentage / (2**16 - 1);

            Token(fromToken).approve(vault, swapAmount);

            ICatalystV1Vault v = ICatalystV1Vault(vault);
            uint256 expectedSwapReturn = v.calcLocalSwap(fromToken, toToken, swapAmount - swapAmount * vaultFee / 10**18);

            vm.prank(v.factoryOwner());
            v.setVaultFee(vaultFee);

            uint256 swapReturnWithFee = v.localSwap(fromToken, toToken, swapAmount, 0);

            assertEq(expectedSwapReturn, swapReturnWithFee, "return after fee not expected4.");
        }
    }

    function test_local_swap_governance_fee(uint16 swapPercentage, uint48 vaultFee, uint48 governanceFee) external virtual {
        vm.assume(uint256(governanceFee) < 75 * 10**16);
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            address fromToken = v._tokenIndexing(0);
            address toToken = v._tokenIndexing(1);
            
            uint256 initialFromTokenBalance = Token(fromToken).balanceOf(v.factoryOwner());

            uint256 swapAmount = getLargestSwap(vault, vault, fromToken, toToken) * swapPercentage / (2**16 - 1);

            Token(fromToken).approve(vault, swapAmount);

            uint256 expectedSwapReturn = v.calcLocalSwap(fromToken, toToken, swapAmount - swapAmount * vaultFee / 10**18);

            vm.prank(v.factoryOwner());
            v.setVaultFee(vaultFee);

            vm.prank(v.factoryOwner());
            v.setGovernanceFee(governanceFee);

            uint256 swapReturnWithFee = v.localSwap(fromToken, toToken, swapAmount, 0);

            assertEq(expectedSwapReturn, swapReturnWithFee, "return after fee not expected2.");

            assertEq(Token(fromToken).balanceOf(v.factoryOwner()) - (initialFromTokenBalance - swapAmount), swapAmount * vaultFee / 10**18 * governanceFee / 10**18, "return after fee not expected3.");
        }
    }
}

