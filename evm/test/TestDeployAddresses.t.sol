// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";

import { DeployContracts } from "../script/DeployContracts.s.sol";

contract TestDeployedCorrectAddresses is Test, DeployContracts {

    function test_check_addresses() external {
        load_config();

        address factory_set = contracts.factory;
        address volatile_mathlib_set = contracts.volatile_mathlib;
        address volatile_template_set = contracts.volatile_template;
        address amplified_mathlib_set = contracts.amplified_mathlib;
        address amplified_template_set = contracts.amplified_template;

        console.log(factory_set);
        console.log(volatile_mathlib_set);
        console.log(volatile_template_set);
        console.log(amplified_mathlib_set);
        console.log(amplified_template_set);

        get = true;
        verify = false;
        vm.startBroadcast(123);

        deployAllContracts(0x0000007aAAC54131e031b3C0D6557723f9365A5B);

        vm.stopBroadcast();


        address factory_new = contracts.factory;
        address volatile_mathlib_new = contracts.volatile_mathlib;
        address volatile_template_new = contracts.volatile_template;
        address amplified_mathlib_new = contracts.amplified_mathlib;
        address amplified_template_new = contracts.amplified_template;

        console.log(factory_new);
        console.log(volatile_mathlib_new);
        console.log(volatile_template_new);
        console.log(amplified_mathlib_new);
        console.log(amplified_template_new);

        assertEq(
            factory_set,
            factory_new,
            "Expected factory address isn't the same as stored"
        );
        assertEq(
            volatile_mathlib_set,
            volatile_mathlib_new,
            "Expected volatile mathlib address isn't the same as stored"
        );
        assertEq(
            volatile_template_set,
            volatile_template_new,
            "Expected volatile template address isn't the same as stored"
        );
        assertEq(
            amplified_mathlib_set,
            amplified_mathlib_new,
            "Expected amplified mathlib address isn't the same as stored"
        );
        assertEq(
            amplified_template_set,
            amplified_template_new,
            "Expected amplified template address isn't the same as stored"
        );
    }
}

