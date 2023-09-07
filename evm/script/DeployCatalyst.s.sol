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
        CatalystFactory factory = new CatalystFactory{salt: 0x61501635278aae8ae4157b2d2c65b1f07f61a0af03ec117d4fee18d6ec435db8}(vm.envAddress("CATALYST_ADDRESS"));
        contracts.factory = address(factory);

        // Deploy Templates
        address volatile_mathlib = address(new CatalystMathVol{salt: 0xb007205e1058308f80830cfb124f8025ec13c921a28ed82df2c82883eb53bbd0}());
        contracts.volatile_mathlib = address(volatile_mathlib);

        address volatile_template = address(
            new CatalystVaultVolatile{salt: 0xa46411ab5dd9f6503cd98b322f8d881cd8aed5aeec4c607fba6e249fada09502}(address(factory), volatile_mathlib)
        );
        contracts.volatile_template = address(volatile_template);

        address amplified_mathlib = address(new CatalystMathAmp{salt: 0x74c361461aa088d1d143e8cdd85276e5d770dc1b3256bcb006c3113b01f0fa78}());
        contracts.amplified_mathlib = address(amplified_mathlib);

        address amplified_template = address(
            new CatalystVaultAmplified{salt: 0xb279fc99702860dd7101fd8cb0bfbc14e8e3d387b152e021e01c70ed56c2f66a}(address(factory), amplified_mathlib)
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

