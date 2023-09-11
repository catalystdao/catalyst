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

contract DeployCatalyst is Script {
    using stdJson for string;

    address[] CCIs;

    bool fillDescriber = false;

    JsonContracts contracts;

    string chain;

    function deployAllContracts() internal {

        // Deploy Factory
        CatalystFactory factory = new CatalystFactory{salt: 0x2ea0e39ef7366f6b504c30f3769f869a827835dc79ad25e94fe3e456cfa35bd8}(vm.envAddress("CATALYST_ADDRESS"));
        contracts.factory = address(factory);

        // Deploy Templates
        address volatile_mathlib = address(new CatalystMathVol{salt: 0xd2c762d8d12ded8f566f25d86ef4cf6fd4ab1beffcc4073adde9ce9ae8ddd803}());
        contracts.volatile_mathlib = address(volatile_mathlib);

        address volatile_template = address(
            new CatalystVaultVolatile{salt: 0xdc8a4cea8d4c6d0f765a266acda548ab542382f65a68e6cb6e371e3746c86cc3}(address(factory), volatile_mathlib)
        );
        contracts.volatile_template = address(volatile_template);

        address amplified_mathlib = address(new CatalystMathAmp{salt: 0x6f3ca6dd912c11c354a6e06318c17f9a71ef6d1d936afa273cea1cf1eff3675f}());
        contracts.amplified_mathlib = address(amplified_mathlib);

        address amplified_template = address(
            new CatalystVaultAmplified{salt: 0x2a55d4f99a04ad2ac8cb16f32934098dd0bdc8562add1fb567e6608cc1524cf7}(address(factory), amplified_mathlib)
        );
        contracts.amplified_template = address(amplified_template);

        // Deploy Registry
        CatalystDescriber catalyst_describer = new CatalystDescriber{salt: bytes32(0)}(vm.envAddress("CATALYST_ADDRESS"));
        contracts.describer = address(catalyst_describer);

        
        CatalystDescriberRegistry describer_registry = new CatalystDescriberRegistry{salt: bytes32(0)}(vm.envAddress("CATALYST_ADDRESS"));
        contracts.describer_registry = address(describer_registry);
        
    }

    function setupDescriber() internal {
        CatalystDescriber catalyst_describer = CatalystDescriber(contracts.describer);

        catalyst_describer.add_vault_factory(contracts.factory);
        catalyst_describer.add_whitelisted_template(contracts.volatile_template, 1);
        catalyst_describer.add_whitelisted_template(contracts.amplified_template, 1);
    }


    function run() external {
        deploy(false);
    }

    function getAddresses() external {
        deploy(true);
    }

    function deploy(bool writeAddresses) internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        deployAllContracts();

        // Fill registry
        setupDescriber();

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

