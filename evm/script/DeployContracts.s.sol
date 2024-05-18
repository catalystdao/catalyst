// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

// Math libs
import { CatalystMathVol } from "../src/registry/CatalystMathVol.sol";
import { CatalystMathAmp } from "../src/registry/CatalystMathAmp.sol";

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
    address factory;
    address volatile_mathlib;
    address volatile_template;
}

contract DeployContracts is Script {
    using stdJson for string;
    
    address private admin;

    bytes32 constant NO_ADDRESS_CODEHASH = bytes32(0);

    bool get;
    bool verify;

    JsonContracts contracts;
    
    function deployFactory(bytes32 salt) internal {
        if ((contracts.factory.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address factory = address(new CatalystFactory{salt: salt}(admin));
        if (verify) require(contracts.factory == factory, "not expected address, factory");
        contracts.factory = factory;
    }

    function deploy_volatile_mathlib(bytes32 salt) internal {
        if ((contracts.volatile_mathlib.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address volatile_mathlib = address(new CatalystMathVol{salt: salt}());
        if (verify) require(contracts.volatile_mathlib == volatile_mathlib, "not expected address, volatile mathlib");
        contracts.volatile_mathlib = volatile_mathlib;
    }

    function deploy_amplified_mathlib(bytes32 salt) internal {
        if ((contracts.amplified_mathlib.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address amplified_mathlib = address(new CatalystMathAmp{salt: salt}());
        if (verify) require(contracts.amplified_mathlib == amplified_mathlib, "not expected address, amplified mathlib");
        contracts.amplified_mathlib = amplified_mathlib;
    }

    function deploy_volatile_template(bytes32 salt) internal {
        if ((contracts.volatile_template.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address volatile_template = address(new CatalystVaultVolatile{salt: salt}(contracts.factory, contracts.volatile_mathlib));
        if (verify) require(contracts.volatile_template == volatile_template, "not expected address, volatile template");
        contracts.volatile_template = volatile_template;
    }

    function deploy_amplified_template(bytes32 salt) internal {
        if ((contracts.amplified_template.codehash != NO_ADDRESS_CODEHASH) && (get == false)) {
            return;
        }
        address amplified_template = address(new CatalystVaultAmplified{salt: salt}(contracts.factory, contracts.amplified_mathlib));
        if (verify) require(contracts.amplified_template == amplified_template, "not expected address, amplified template");
        contracts.amplified_template = amplified_template;
    }

    function load_config() internal {
        string memory pathRoot = vm.projectRoot();
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));
    }

    function deployAllContracts(address admin_) internal {
        load_config();

        admin = admin_;

        deployFactory(bytes32(0x05624c1ea3c3ccda5de1f660f08f273485a6705c2a16ba15faa6dc4bc6ab08e7));

        deploy_volatile_mathlib(bytes32(0xe5a3f4676abc23027a0cd4359d4c97e42f4220eafcab86d68659524867949a45));
        deploy_amplified_mathlib(bytes32(0xb6b5fa2553b5e78058a72aa1df20ead9538ff71011621b994ed3114a4e573361));

        deploy_volatile_template(bytes32(0x8add0c36c676f99a85b05f50ebf5306c7e0ca540a6985e18a339013c1f1dbc38));
        deploy_amplified_template(bytes32(0xe902feb8d6b508a0bf6dabc08b2fd6a8ea2e64253f957e95108794a339201fbd));
    }
}

