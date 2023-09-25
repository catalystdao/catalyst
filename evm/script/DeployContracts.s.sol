// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

// Math libs
import { CatalystMathVol } from "../src/registry/CatalystMathVol.sol";
import { CatalystMathAmp } from "../src/registry/CatalystMathAmp.sol";

// Registry
import { CatalystDescriber } from "../src/registry/CatalystDescriber.sol";
import { CatalystDescriberRegistry } from "../src/registry/CatalystDescriberRegistry.sol";

// Router
import { CatalystRouter } from "../src/router/CatalystRouter.sol";
import { RouterParameters } from "../src/router/base/RouterImmutables.sol";

// Core Catalyst
import { CatalystFactory } from "../src/CatalystFactory.sol";
import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";

struct JsonContracts {
    address amplified_mathlib;
    address amplified_template;
    address describer;
    address describer_registry;
    address factory;
    address volatile_mathlib;
    address volatile_template;
}

contract DeployContracts is Script {
    using stdJson for string;
    address private admin;

    bytes32 constant NO_ADDRESS_CODEHASH = 0x0000000000000000000000000000000000000000000000000000000000000000;

    JsonContracts contracts;
    
    function deployFactory(bytes32 salt) internal {
        if (contracts.factory.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.factory = address(new CatalystFactory{salt: salt}(admin));
    }

    function deploy_volatile_mathlib(bytes32 salt) internal {
        if (contracts.volatile_mathlib.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.volatile_mathlib = address(new CatalystMathVol{salt: salt}());
    }

    function deploy_amplified_mathlib(bytes32 salt) internal {
        if (contracts.amplified_mathlib.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.amplified_mathlib = address(new CatalystMathAmp{salt: salt}());
    }

    function deploy_volatile_template(bytes32 salt) internal {
        if (contracts.volatile_template.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.volatile_template = address(new CatalystVaultVolatile{salt: salt}(contracts.factory, contracts.volatile_mathlib));
    }

    function deploy_amplified_template(bytes32 salt) internal {
        if (contracts.amplified_template.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.amplified_template = address(new CatalystVaultAmplified{salt: salt}(contracts.factory, contracts.amplified_mathlib));
    }

    function deploy_describer(bytes32 salt) internal {
        if (contracts.describer.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.describer = address(new CatalystDescriber{salt: salt}(admin));
    }

    function deploy_registry(bytes32 salt) internal {
        if (contracts.describer_registry.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        contracts.describer_registry = address(new CatalystDescriberRegistry{salt: salt}(admin));
    }

    function deployAllContracts(address admin_) internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));

        admin = admin_;

        deployFactory(bytes32(uint256(10)));

        deploy_volatile_mathlib(bytes32(uint256(10)));
        deploy_amplified_mathlib(bytes32(uint256(10)));

        deploy_volatile_template(bytes32(uint256(10)));
        deploy_amplified_template(bytes32(uint256(10)));

        // Deploy Registry
        deploy_describer(bytes32(uint256(10)));
        deploy_registry(bytes32(uint256(10)));

        // Fill registry
        setupDescriber();
    }

    function setupDescriber() internal {
        CatalystDescriber catalyst_describer = CatalystDescriber(contracts.describer);
        CatalystDescriberRegistry catalyst_registry = CatalystDescriberRegistry(contracts.describer_registry);

        if (catalyst_describer.get_num_vault_factories() != 0) {
            return;
        }

        catalyst_registry.add_describer(address(catalyst_describer));

        catalyst_describer.add_vault_factory(contracts.factory);
        catalyst_describer.add_whitelisted_template(contracts.volatile_template, 1);
        catalyst_describer.add_whitelisted_template(contracts.amplified_template, 1);
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
}

