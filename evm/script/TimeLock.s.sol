// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";

import { BaseMultiChainDeployer } from "./BaseMultiChainDeployer.s.sol";

import { TimelockController } from "openzeppelin-contracts/contracts/governance/TimelockController.sol";

contract DeployTimelock is BaseMultiChainDeployer {

    uint256 MIN_DELAY = 1 days;
    
    function _deployTimelockController(bytes32 salt, uint256 minDelay, address[] memory proposers, address[] memory executors, address admin) internal returns(TimelockController) {
        return new TimelockController{salt: salt}(minDelay, proposers, executors, admin);
    }

    function deployTimelockController(bytes32 salt, address initalProposer) public returns(TimelockController) {
        address[] memory proposers = new address[](1);
        proposers[0] = initalProposer;
        address[] memory executors = new address[](1);
        executors[0] = address(0); // Let anyone execute.
        address admin = address(0);

        return _deployTimelockController(salt, MIN_DELAY, proposers, executors, admin);
    }

    function run() public {
        bytes32 salt = bytes32(0x67e6613ce8cbbe2cbdf83a49a42f7506ba4f34ef5ce61e83c3935e85cac6a4a6);
        address initalProposer = address(0xE759cBa7dE5bF6E024BcbdD01941fc3b1713D2FC);
        deployTimelockController(salt, initalProposer);
    }
}

