// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestWithdrawEverything is TestCommon, AVaultInterfaces {

    /// @notice Test that you can withdraw everything.
    function test_withdraw_everything() external {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault v = ICatalystV1Vault(vault);
            
            uint256[] memory minouts = new uint256[](3);
            // Withdraw everything.
            v.withdrawAll(Token(vault).balanceOf(address(this)), minouts);

            // ensure that the vault is empty.
            
            uint256 j = 0;
            while (true) {
                address tkn = v._tokenIndexing(j);
                if (tkn == address(0)) break;
                uint256 vaultBalance = Token(tkn).balanceOf(vault);
                assertEq(vaultBalance, 0, "Vault is not empty");

                ++j;
            }
        }
    }
}