// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../../src/ICatalystV1Vault.sol";
import {Token} from "../mocks/token.sol";
import "../../src/utils/FixedPointMathLib.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

abstract contract TestLocalswapMinout is Test, AVaultInterfaces {
    
    function test_error_localswap_minout(uint56 amount, uint56 minout) external virtual {
        vm.assume(amount != 0);
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address swapVault = vaults[i];

            address fromToken = ICatalystV1Vault(swapVault)._tokenIndexing(0);
            address toToken = ICatalystV1Vault(swapVault)._tokenIndexing(1);

            ICatalystV1Vault v = ICatalystV1Vault(swapVault);

            Token(fromToken).approve(address(v), amount);

            uint256 initialBalance = Token(toToken).balanceOf(address(this));
            uint256 out = v.calcLocalSwap(fromToken, toToken, amount);

            if (out < minout) {
                vm.expectRevert(
                    abi.encodeWithSignature(
                        "ReturnInsufficient(uint256,uint256)",
                        out, minout
                    )
                );
                v.localSwap(fromToken, toToken, amount, minout);
            } else {
                v.localSwap(fromToken, toToken, amount, minout);

                require(out >= minout, "Minout not working");
                assertEq(
                    Token(toToken).balanceOf(address(this)) - initialBalance,
                    out,
                    "output not as expected"
                );
            }

            
        }
    }
}

