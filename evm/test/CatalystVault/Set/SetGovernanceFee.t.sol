// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestSetGovernanceFee is Test, AVaultInterfaces {

    function test_set_governance_fee(uint48 governanceFee) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            ICatalystV1Vault v = ICatalystV1Vault(vault);

            assertEq(
                v._governanceFeeShare(),
                0,
                "valut fee not 0"
            );

            vm.prank(v.factoryOwner());
            v.setGovernanceFee(governanceFee);

            assertEq(
                v._governanceFeeShare(),
                governanceFee,
                "valut fee not updated"
            );
        }
    }

    function test_error_non_admin_set_governance_fee(address impersonator, uint48 governanceFee) external virtual {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            ICatalystV1Vault v = ICatalystV1Vault(vault);
            if (impersonator != v.factoryOwner()) continue;

            vm.prank(impersonator);
            vm.expectRevert(bytes(""));
            v.setGovernanceFee(governanceFee);
        }
    }




    function test_set_governance_fee_not_too_large() external virtual {
        address[] memory vaults = getTestConfig();

        uint256 vaultFee = 75 * 10**16;

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            ICatalystV1Vault v = ICatalystV1Vault(vault);

            vm.prank(v.factoryOwner());
            v.setGovernanceFee(vaultFee);
        }
    }

    function test_error_set_governance_fee_too_large() external virtual {
        address[] memory vaults = getTestConfig();

        uint256 vaultFee = 75 * 10**16 + 1;

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            ICatalystV1Vault v = ICatalystV1Vault(vault);

            vm.prank(v.factoryOwner());
            vm.expectRevert(bytes(""));
            v.setGovernanceFee(vaultFee);
        }
    }
}

