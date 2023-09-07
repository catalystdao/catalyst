// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../../src/ICatalystV1Vault.sol";
import {Token} from "../mocks/token.sol";
import "../../src/utils/FixedPointMathLib.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

abstract contract TestSetVaultFee is Test, AVaultInterfaces {
    function test_set_fee(uint48 vaultFee) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            ICatalystV1Vault v = ICatalystV1Vault(vault);

            assertEq(
                v._vaultFee(),
                0,
                "valut fee not 0"
            );
            
            vm.prank(v.factoryOwner());
            v.setVaultFee(vaultFee);

            assertEq(
                v._vaultFee(),
                vaultFee,
                "valut fee not updated"
            );
        }
    }

    function test_error_non_admin_set_fee(address impersonator, uint48 vaultFee) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            ICatalystV1Vault v = ICatalystV1Vault(vault);
            if (impersonator != v.factoryOwner()) continue;

            vm.prank(impersonator);
            vm.expectRevert(bytes(""));
            v.setVaultFee(vaultFee);
        }
    }
}

