// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Permit2 } from "../lib/permit2/src/Permit2.sol";

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


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";

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
    address private admin;

    JsonContracts contracts;
    
    function deployFactory(bytes32 salt) internal {
        contracts.factory = address(new CatalystFactory{salt: salt}(admin));
    }

    function deploy_volatile_mathlib(bytes32 salt) internal {
        contracts.volatile_mathlib = address(new CatalystMathVol{salt: salt}());
    }

    function deploy_amplified_mathlib(bytes32 salt) internal {
        contracts.amplified_mathlib = address(new CatalystMathAmp{salt: salt}());
    }

    function deploy_volatile_template(bytes32 salt) internal {
        contracts.volatile_template = address(new CatalystVaultVolatile{salt: salt}(contracts.factory, contracts.volatile_mathlib));
    }

    function deploy_amplified_template(bytes32 salt) internal {
        contracts.amplified_template = address(new CatalystVaultAmplified{salt: salt}(contracts.factory, contracts.amplified_mathlib));
    }

    function deploy_describer(bytes32 salt) internal {
        contracts.describer = address(new CatalystDescriber{salt: salt}(admin));
    }

    function deploy_registry(bytes32 salt) internal {
        contracts.describer_registry = address(new CatalystDescriberRegistry{salt: salt}(admin));
    }

    function deployAllContracts(address admin_) internal {
        admin = admin_;

        deployFactory(0x2ea0e39ef7366f6b504c30f3769f869a827835dc79ad25e94fe3e456cfa35bd8);

        deploy_volatile_mathlib(0xd2c762d8d12ded8f566f25d86ef4cf6fd4ab1beffcc4073adde9ce9ae8ddd803);
        deploy_amplified_mathlib(0x6f3ca6dd912c11c354a6e06318c17f9a71ef6d1d936afa273cea1cf1eff3675f);

        deploy_volatile_template(0xdc8a4cea8d4c6d0f765a266acda548ab542382f65a68e6cb6e371e3746c86cc3);
        deploy_amplified_template(0x2a55d4f99a04ad2ac8cb16f32934098dd0bdc8562add1fb567e6608cc1524cf7);

        // Deploy Registry
        deploy_describer(bytes32(0));
        deploy_registry(bytes32(0));

        // Fill registry
        setupDescriber();
    }

    function setupDescriber() internal {
        CatalystDescriber catalyst_describer = CatalystDescriber(contracts.describer);

        catalyst_describer.add_vault_factory(contracts.factory);
        catalyst_describer.add_whitelisted_template(contracts.volatile_template, 1);
        catalyst_describer.add_whitelisted_template(contracts.amplified_template, 1);
    }

    function getAddresses() external {
        address admin_ = vm.envAddress("CATALYST_ADDRESS");

        deployAllContracts(admin_);

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

