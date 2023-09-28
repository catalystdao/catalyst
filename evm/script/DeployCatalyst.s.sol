// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

import { BaseMultiChainDeployer } from "./BaseMultiChainDeployer.s.sol";
import { JsonContracts, DeployContracts } from "./DeployContracts.s.sol";

// Registry
import { CatalystDescriber } from "../src/registry/CatalystDescriber.sol";
import { CatalystDescriberRegistry } from "../src/registry/CatalystDescriberRegistry.sol";


contract DeployCatalyst is BaseMultiChainDeployer, DeployContracts {
    using stdJson for string;

    function deploy() iter_chains(chain_list) broadcast external {
        verify = true;
        address admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);

        setupDescriber();
    }
    function deploy_legacy() iter_chains(chain_list_legacy) broadcast external {
        verify = true;
        address admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);

        setupDescriber();
    }

    function fund_address(address toFund) iter_chains(chain_list) broadcast external {
        fund(toFund, 0.2*10**18);
    }

    function fund_address_legacy(address toFund) iter_chains(chain_list_legacy) broadcast external {
        fund(toFund, 0.2*10**18);
    }

    function setupDescriber() internal {
        CatalystDescriber catalyst_describer = CatalystDescriber(contracts.describer);
        CatalystDescriberRegistry catalyst_registry = CatalystDescriberRegistry(contracts.describer_registry);

        if (catalyst_describer.get_num_vault_factories() == 0) {
            catalyst_describer.add_vault_factory(contracts.factory);
        }
        if (catalyst_describer.get_num_whitelisted_templates() == 0) {
            catalyst_describer.add_whitelisted_template(contracts.volatile_template, 1);
            catalyst_describer.add_whitelisted_template(contracts.amplified_template, 1);
        }

        if (catalyst_registry.catalyst_version() == 0) {
            catalyst_registry.add_describer(address(catalyst_describer));
        }
    }

    function getAddresses() external {
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
        vm.serializeAddress(obj, "describer", contracts.describer);
        vm.serializeAddress(obj, "describer_registry", contracts.describer_registry);
        vm.serializeAddress(obj, "factory", contracts.factory);
        vm.serializeAddress(obj, "volatile_mathlib", contracts.volatile_mathlib);
        string memory finalJson = vm.serializeAddress(obj, "volatile_template", contracts.volatile_template);

        vm.writeJson(finalJson, pathToContractConfig, string.concat(".contracts"));
    }


    function _regStore() iter_chains(chain_list) internal {
        CatalystDescriber desc =CatalystDescriber(contracts.describer);

        console.log("factories");
        address[] memory facts = desc.get_vault_factories();
        for (uint256 i = 0; i < facts.length; ++i) {
            console.logAddress(facts[i]);
        }

        console.log("templates");
        address[] memory templates = desc.get_whitelisted_templates();
        for (uint256 i = 0; i < templates.length; ++i) {
            console.logAddress(templates[i]);
        }

        console.log("cci");
        CatalystDescriber.CrossChainInterface[] memory cci = desc.get_whitelisted_CCI();
        for (uint256 i = 0; i < cci.length; ++i) {
            console.log(cci[i].version);
            console.logAddress(cci[i].cci);
        }
    }

    function regStore() external {
        load_config();

        _regStore();
    }


    function regStore_legacy() external {
        load_config();

        _regStore();
    }
}

