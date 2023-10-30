// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

import { BaseMultiChainDeployer } from "./BaseMultiChainDeployer.s.sol";
import { JsonContracts, DeployContracts } from "./DeployContracts.s.sol";


contract DeployCatalyst is BaseMultiChainDeployer, DeployContracts {
    using stdJson for string;

    address private admin;

    function deploy() iter_chains(chain_list) broadcast external {
        verify = true;
        admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);
    }
    function deploy_legacy() iter_chains(chain_list_legacy) broadcast external {
        verify = true;
        admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);
    }

    function fund_address(address toFund) iter_chains(chain_list) broadcast external {
        fund(toFund, 0.2*10**18);
    }

    function fund_address_legacy(address toFund) iter_chains(chain_list_legacy) broadcast external {
        fund(toFund, 0.2*10**18);
    }

    function getAddresses() external {
        get = true;
        address admin_ = vm.envAddress("CATALYST_ADDRESS");
        uint256 pk = vm.envUint("CATALYST_DEPLOYER");

        vm.startBroadcast(pk);

        deployAllContracts(admin_);

        vm.stopBroadcast();

        // Save json
        writeToJson();
    }

    function writeToJson() internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory obj = "";

        vm.serializeAddress(obj, "amplified_mathlib", contracts.amplified_mathlib);
        vm.serializeAddress(obj, "amplified_template", contracts.amplified_template);
        vm.serializeAddress(obj, "factory", contracts.factory);
        vm.serializeAddress(obj, "volatile_mathlib", contracts.volatile_mathlib);
        string memory finalJson = vm.serializeAddress(obj, "volatile_template", contracts.volatile_template);

        vm.writeJson(finalJson, pathToContractConfig, string.concat(".contracts"));
    }
}

