// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";

contract FundAddresses is Script {

    function run() external {
        uint256 CATALYST_AMOUNT = 3 ether;
        address CATALYST_ADDRESS = vm.envAddress("CATALYST_ADDRESS");

        uint256 WGAS_AMOUNT = 0.5 ether;
        address WGAS_DEPLOYER_ADDRESS = vm.envAddress("WGAS_DEPLOYER_ADDRESS");

        vm.startBroadcast(vm.envUint("BASE_DEPLOYER_KEY"));

        if (CATALYST_AMOUNT > 0) payable(CATALYST_ADDRESS)      .transfer(CATALYST_AMOUNT);
        if (WGAS_AMOUNT > 0)    payable(WGAS_DEPLOYER_ADDRESS)  .transfer(WGAS_AMOUNT);

        vm.stopBroadcast();
    }
}

