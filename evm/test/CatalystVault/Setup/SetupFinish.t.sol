// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../../mocks/token.sol";
import {AVaultInterfaces} from "../AVaultInterfaces.t.sol";

abstract contract TestSetupFinish is TestCommon, AVaultInterfaces {
    event FinishSetup();

    function test_finish_setup() external {
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            assertEq(
                ICatalystV1Vault(vault).ready(),
                false,
                "vault ready before finish setup"
            );

            ICatalystV1Vault(vault).finishSetup();

            assertEq(
                ICatalystV1Vault(vault).ready(),
                true,
                "vault not after finish setup"
            );
        }
    }


    // Authority and state tests *****************************************************************************************************


    function test_finish_setup_unauthorized(address alice) external {
        vm.assume(alice != address(this));
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            
            vm.prank(alice);
            vm.expectRevert(bytes(""));
            ICatalystV1Vault(vault).finishSetup();
        }
    }


    function test_finish_setup_twice(address alice) external {
        vm.assume(alice != address(this));
        address[] memory vaults = getTestConfig();

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            assertEq(
                address(this),
                ICatalystV1Vault(vault)._setupMaster(),
                "Setup master not deployer"
            );
            
            vm.expectEmit();
            emit FinishSetup();
            ICatalystV1Vault(vault).finishSetup();

            assertEq(
                address(0),
                ICatalystV1Vault(vault)._setupMaster(),
                "Setup master not set to address(0)"
            );

            vm.expectRevert(bytes(""));
            ICatalystV1Vault(vault).finishSetup();
        }
    }
}
