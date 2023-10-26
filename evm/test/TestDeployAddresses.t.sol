// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";

import { DeployContracts } from "../script/DeployContracts.s.sol";

contract TestDeployedCorrectAddresses is Test, DeployContracts {

    function test_check_addresses() external {
        load_config();

        console.log(contracts.factory);
        console.log(contracts.volatile_mathlib);
        console.log(contracts.volatile_template);
        console.log(contracts.amplified_mathlib);
        console.log(contracts.amplified_template);

        get = true;
        verify = true;
        vm.startBroadcast(123);

        deployAllContracts(0x0000007aAAC54131e031b3C0D6557723f9365A5B);

        vm.stopBroadcast();

        console.log(contracts.factory);
        console.log(contracts.volatile_mathlib);
        console.log(contracts.volatile_template);
        console.log(contracts.amplified_mathlib);
        console.log(contracts.amplified_template);
    }
}

