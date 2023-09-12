// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";

import { BaseMultiChainDeployer } from "./BaseMultiChainDeployer.s.sol";
import { DeployContracts } from "./DeployContracts.s.soL";


contract DeployCatalyst is BaseMultiChainDeployer, DeployContracts {

    function deploy() iter_chains(chain_list) broadcast external {
        address admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);
    }
    function deploy_legacy() iter_chains(chain_list_legacy) broadcast external {
        address admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);
    }
}

