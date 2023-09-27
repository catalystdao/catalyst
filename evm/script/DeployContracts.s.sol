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

    bool verify;

    JsonContracts contracts;
    
    function deployFactory(bytes32 salt) internal {
        if (contracts.factory.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address factory = address(new CatalystFactory{salt: salt}(admin));
        if (verify) require(contracts.factory == factory, "not expected address, factory");
        contracts.factory = factory;
    }

    function deploy_volatile_mathlib(bytes32 salt) internal {
        if (contracts.volatile_mathlib.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address volatile_mathlib = address(new CatalystMathVol{salt: salt}());
        if (verify) require(contracts.volatile_mathlib == volatile_mathlib, "not expected address, volatile mathlib");
        contracts.volatile_mathlib = volatile_mathlib;
    }

    function deploy_amplified_mathlib(bytes32 salt) internal {
        if (contracts.amplified_mathlib.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address amplified_mathlib = address(new CatalystMathAmp{salt: salt}());
        if (verify) require(contracts.amplified_mathlib == amplified_mathlib, "not expected address, amplified mathlib");
        contracts.amplified_mathlib = amplified_mathlib;
    }

    function deploy_volatile_template(bytes32 salt) internal {
        if (contracts.volatile_template.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address volatile_template = address(new CatalystVaultVolatile{salt: salt}(contracts.factory, contracts.volatile_mathlib));
        if (verify) require(contracts.volatile_template == volatile_template, "not expected address, volatile template");
        contracts.volatile_template = volatile_template;
    }

    function deploy_amplified_template(bytes32 salt) internal {
        if (contracts.amplified_template.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address amplified_template = address(new CatalystVaultAmplified{salt: salt}(contracts.factory, contracts.amplified_mathlib));
        if (verify) require(contracts.amplified_template == amplified_template, "not expected address, amplified template");
        contracts.amplified_template = amplified_template;
    }

    function deploy_describer(bytes32 salt) internal {
        if (contracts.describer.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address describer = address(new CatalystDescriber{salt: salt}(admin));
        if (verify) require(contracts.describer == describer, "not expected address, describer");
        contracts.describer = describer;        
    }

    function deploy_registry(bytes32 salt) internal {
        if (contracts.describer_registry.codehash != NO_ADDRESS_CODEHASH) {
            return;
        }
        address describer_registry = address(new CatalystDescriberRegistry{salt: salt}(admin));
        if (verify) require(contracts.describer_registry == describer_registry, "not expected address, describer_registry");
        contracts.describer_registry = describer_registry;
    }

    function deployAllContracts(address admin_) internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));

        admin = admin_;

        deployFactory(bytes32(uint256(9)));

        deploy_volatile_mathlib(bytes32(uint256(9)));
        deploy_amplified_mathlib(bytes32(uint256(9)));

        deploy_volatile_template(bytes32(uint256(9)));
        deploy_amplified_template(bytes32(uint256(9)));

        // Deploy Registry
        deploy_describer(bytes32(uint256(9)));
        deploy_registry(bytes32(uint256(9)));

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

