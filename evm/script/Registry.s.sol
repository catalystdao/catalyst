// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

import { BaseMultiChainDeployer } from "./BaseMultiChainDeployer.s.sol";
import { JsonContracts } from "./DeployContracts.s.sol";
import { CatalystDescriber } from "../src/registry/CatalystDescriber.sol";
import { CatalystDescriberRegistry } from "../src/registry/CatalystDescriberRegistry.sol";
import { CatalystLens } from "../src/token-lens/CatalystLens.sol";

struct JsonRegistry {
    address describer;
    address describer_registry;
    address lens;
}

contract Registry is BaseMultiChainDeployer {
    using stdJson for string;
    
    bytes32 constant NO_ADDRESS_CODEHASH = bytes32(0);

    bool get;
    bool verify;

    string config_interfaces;

    JsonRegistry registry;
    JsonContracts contracts;

    address private admin;

    function deploy_describer(bytes32 salt) internal {
        if ((registry.describer.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address describer = address(new CatalystDescriber{salt: salt}(admin));
        if (verify) require(registry.describer == describer, "not expected address, describer");
        registry.describer = describer;        
    }

    function deploy_registry(bytes32 salt) internal {
        if ((registry.describer_registry.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address describer_registry = address(new CatalystDescriberRegistry{salt: salt}(admin));
        if (verify) require(registry.describer_registry == describer_registry, "not expected address, describer_registry");
        registry.describer_registry = describer_registry;
    }

    function deploy_lens(bytes32 salt) internal {
        if ((registry.lens.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address lens = address(new CatalystLens{salt: salt}());
        if (verify) require(registry.lens == lens, "not expected address, lens");
        registry.lens = lens;
    }

    function _deploy() internal {
        load_config();
        deploy_describer(bytes32(0));
        deploy_registry(bytes32(0));
        deploy_lens(bytes32(0));

        setRegistry("v1");
        setDescriber();
    }

    function deploy(string[] memory chains) iter_chains_string(chains) broadcast external {
        verify = true;
        admin = vm.envAddress("CATALYST_ADDRESS");
        _deploy();
    }

    function deployAll() iter_chains(chain_list) broadcast external {
        verify = true;
        admin = vm.envAddress("CATALYST_ADDRESS");
        _deploy();
    }

    function deployAllLegacy() iter_chains(chain_list_legacy) broadcast external {
        verify = true;
        admin = vm.envAddress("CATALYST_ADDRESS");
        _deploy();
    }
    
    function getAddresses() external {
        get = true;
        admin = vm.envAddress("CATALYST_ADDRESS");

        vm.startBroadcast(uint256(1));

        deploy_describer(bytes32(0));
        deploy_registry(bytes32(0));
        deploy_lens(bytes32(0));

        vm.stopBroadcast();

        writeToJson();
    }

    function setRegistry(address reg_address, address describer, string memory version) iter_chains(chain_list) broadcast external {
        CatalystDescriberRegistry reg = CatalystDescriberRegistry(reg_address);

        reg.modifyDescriber(describer, version);
    }

    function setRegistryLegacy(address reg_address, address describer, string memory version) iter_chains(chain_list_legacy) broadcast external {
        CatalystDescriberRegistry reg = CatalystDescriberRegistry(reg_address);

        reg.modifyDescriber(describer, version);
    }

    function setRegistry(string memory version) internal {
        CatalystDescriberRegistry reg = CatalystDescriberRegistry(registry.describer_registry);

        // Check what is the current describer.
        CatalystDescriberRegistry.AddressAndVersion[] memory describers = reg.getVaultDescribers();
        bool contains = false;
        for (uint256 i = 0; i < describers.length; ++i) {
            if (describers[i].addr == registry.describer) contains = true;
        }
        if (!contains) reg.modifyDescriber(registry.describer, version);
    }

    function setDescriber() internal {
        CatalystDescriber desc = CatalystDescriber(registry.describer);
        // Set (or update) the templates
        address current_volatile_template = desc.versionToTemplate("volatile");
        if (current_volatile_template != contracts.volatile_template) desc.modifyWhitelistedTemplate(contracts.volatile_template, "volatile");

        address current_amplified_template = desc.versionToTemplate("amplified");
        if (current_amplified_template != contracts.amplified_template) desc.modifyWhitelistedTemplate(contracts.amplified_template, "amplified");

        // Set (or update) the factory.
        address current_factory = desc.versionToFactory("v1");
        if (current_factory != contracts.factory) desc.modifyWhitelistedFactory(contracts.factory, "v1");

        // Set (or update) the cross-chain interfaces
        string[] memory availableInterfaces = vm.parseJsonKeys(config_interfaces, string.concat("$"));
        for (uint256 i = 0; i < availableInterfaces.length; ++i) {
            string memory incentiveVersion = availableInterfaces[i];
            if (!vm.keyExists(config_interfaces, string.concat(".", incentiveVersion, ".", currentChainKey))) {
                continue;
            }
            address excepted_cci = abi.decode(config_interfaces.parseRaw(string.concat(".", incentiveVersion, ".", currentChainKey, ".interface")), (address));
            
            address current_cci = desc.versionToCCI(incentiveVersion);
            if (current_cci != excepted_cci) desc.modifyWhitelistedCCI(excepted_cci, incentiveVersion);
        }
    }


    function load_config() internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory pathToInterfacesConfig = string.concat(pathRoot, "/script/config/config_interfaces.json");
        config_interfaces = vm.readFile(pathToInterfacesConfig);

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        registry = abi.decode(config_contract.parseRaw(string.concat(".registry")), (JsonRegistry));
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));

    }

    function writeToJson() internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory obj = "";

        vm.serializeAddress(obj, "describer", registry.describer);
        vm.serializeAddress(obj, "lens", registry.lens);
        string memory finalJson = vm.serializeAddress(obj, "describer_registry", registry.describer_registry);

        vm.writeJson(finalJson, pathToContractConfig, string.concat(".registry"));
    }
}

