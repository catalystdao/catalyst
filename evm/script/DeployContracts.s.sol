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

        deployFactory(bytes32(0xe810ad7329bc18a3661da69243c260bddf93c02cb86f51d82cef8f77664b1dc1));

        deploy_volatile_mathlib(bytes32(0xeadb0c4c623a1504e49ea876afaf2e1a24a663e81e05ac04fc82ab1a53974533));
        deploy_amplified_mathlib(bytes32(0x178f2263b6b48c743c92919210db1f9ffdd16b679f9a49a60db831ee745c1b53));

        deploy_volatile_template(bytes32(0xab571b0631b64d37caa8e7d85912d505bf434d66e2b9df4088447821dac531d8));
        deploy_amplified_template(bytes32(0x4aa36fb7e5e332061b6ad20a3b0de53dd4cf41391c9bf553367ba5165b60c5cd));
    }
}

