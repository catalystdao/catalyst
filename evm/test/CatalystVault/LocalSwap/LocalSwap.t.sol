// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

interface TF {
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool);
}

abstract contract TestLocalswap is Test, AVaultInterfaces {
    
    uint256 private constant MARGIN_NUM = 1;
    uint256 private constant MARGIN_DENOM = 1e12;

    function t_only_localswap(address vault, uint256 amount, address fromAsset, address toAsset) internal {
        ICatalystV1Vault v = ICatalystV1Vault(vault);

        Token(fromAsset).approve(vault, amount);

        v.localSwap(fromAsset, toAsset, amount, 0);
    }

    function t_localswap(address[] memory vaults, address swapVault, uint256 amount, address fromAsset, address toAsset) internal {
        uint256 initial_invariant = invariant(vaults);

        t_only_localswap(swapVault, amount, fromAsset, toAsset);

        uint256 after_invariant = invariant(vaults);

        // We allow upto a (very small)% decrease. If the pool size is $1 million million ($1e12), then we are okay with losing 1$.
        if (after_invariant < initial_invariant) {
            assertGt(
                initial_invariant * MARGIN_NUM / MARGIN_DENOM,
                initial_invariant - after_invariant,
                "Swap error beyond margin found"
            );
        }
    }

    function test_local_swap_invariance(uint32 swapSizePercentage) external virtual {
        address[] memory vaults = getTestConfig();

        address vault = vaults[0];
        ICatalystV1Vault v = ICatalystV1Vault(vault);

        address fromToken = v._tokenIndexing(0);
        address toToken = v._tokenIndexing(1);

        uint256 swapAmount = getLargestSwap(vault, vault, fromToken, toToken) * uint256(swapSizePercentage) / (2**32 - 1);

        t_localswap(vaults, vault, swapAmount, fromToken, toToken);
    }

    function test_local_swap_invariance_same_token(uint32 swapSizePercentage) external virtual {
        address[] memory vaults = getTestConfig();

        address vault = vaults[0];

        address sameToken = ICatalystV1Vault(vault)._tokenIndexing(0);

        uint256 swapAmount = getLargestSwap(vault, vault, sameToken, sameToken) * uint256(swapSizePercentage) / (2**32 - 1);

        t_localswap(vaults, vault, swapAmount, sameToken, sameToken);
    }

    function test_local_swap_zero() external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            address fromToken = v._tokenIndexing(0);
            address toToken = v._tokenIndexing(1);


            vm.expectRevert(
                abi.encodeWithSignature(
                    "ReturnInsufficient(uint256,uint256)",
                    0, 1
                )
            );
            v.localSwap(fromToken, toToken, 0, 1);

            uint256 balanceBefore = Token(toToken).balanceOf(vault);
            v.localSwap(fromToken, toToken, 0, 0);
            uint256 balanceAfter = Token(toToken).balanceOf(vault);

            assertEq(balanceBefore, balanceAfter, "0 value swap changed pool balances");
        }
    }

    function test_no_allowance_set(uint8 amount) external virtual {
        vm.assume(amount != 0);
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            address fromToken = v._tokenIndexing(0);
            address toToken = v._tokenIndexing(1);


            vm.expectRevert(abi.encodePacked(uint32(0x7939f424))); // TRANSFER_FROM_FAILED
            v.localSwap(fromToken, toToken, amount, 0);
        }
    }

    function test_local_swap_withdraw_tokens(uint8 amount) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            address fromToken = v._tokenIndexing(0);
            address toToken = v._tokenIndexing(1);


            Token(fromToken).approve(vault, amount);
            uint256 balanceBefore = Token(fromToken).balanceOf(address(this));
            v.localSwap(fromToken, toToken, amount, 0);
            uint256 balanceAfter = Token(fromToken).balanceOf(address(this));
            assertEq(balanceBefore - balanceAfter, amount);
        }
    }

    function test_local_swap_transfer_from_called(uint8 amount) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);

            address fromToken = v._tokenIndexing(0);
            address toToken = v._tokenIndexing(1);


            Token(fromToken).approve(vault, amount);

            vm.expectCall(
                fromToken,
                abi.encodeCall(
                    TF.transferFrom,
                    (
                        address(this),
                        vault,
                        amount
                    )
                )
            );
            v.localSwap(fromToken, toToken, amount, 0);
        }
    }
}

