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
import { CatalystGARPInterface } from "../src/CatalystGARPInterface.sol";
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
    address permit2;
    address router;
    address volatile_mathlib;
    address volatile_template;
}

contract DeployCatalyst is Script {
    using stdJson for string;

    // string config_contract;
    // string config_chain;

    address[] CCIs;

    address WGAS;

    bool fillDescriber = false;

    JsonContracts contracts;

    string chain;
    bytes32 chainIdentifier;

    error NoWrappedGasTokenFound();

    function getOrDeployPermit2() internal returns(address permit2) {
        permit2 = contracts.permit2;
        if (permit2 != address(0)) return permit2;

        permit2 = address(new Permit2{salt: bytes32(0)}());
        contracts.permit2 = permit2;
    }

    function getGasToken() internal returns(address wrappedGas) {
        wrappedGas = WGAS;

        if (wrappedGas == address(0)) {
            revert NoWrappedGasTokenFound();
        }
    }

    function whitelistAllCCIs(CatalystDescriber catalyst_describer) internal {
        // read config_interfaces
        string memory pathRoot = vm.projectRoot();
        string memory pathToInterfacesConfig = string.concat(pathRoot, "/script/config/config_interfaces.json");
        string memory config_interfaces = vm.readFile(pathToInterfacesConfig);

        string[] memory availableInterfaces = abi.decode(config_interfaces.parseRaw(string.concat(".", chain, ".available")), (string[]));

        for (uint256 i = 0; i < availableInterfaces.length; ++i) {
            string memory interfaceVersion = availableInterfaces[i];
            address interfaceAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", chain, ".", interfaceVersion, ".interface")), (address));
            CCIs.push(interfaceAddress);
        }


        for (uint256 i = 0; i < CCIs.length; ++i) {
            catalyst_describer.add_whitelisted_cci(CCIs[i]);
        }
    }

    function deployAllContracts() internal {

        // Deploy Factory
        CatalystFactory factory = CatalystFactory(contracts.factory);
        if (address(factory) == address(0)) {
            factory = new CatalystFactory{salt: bytes32(uint256(17913527253544052377148342935207663648266424961550501187556387275812090712216))}(vm.envAddress("CATALYST_ADDRESS"));
        }
        contracts.factory = address(factory);

        // Deploy Templates
        address volatile_mathlib = contracts.volatile_mathlib;
        if (volatile_mathlib == address(0)) volatile_mathlib = address(new CatalystMathVol{salt: bytes32(uint256(85598404464591085278359771032523095787540092714401110853070981275698672848766))}());
        contracts.volatile_mathlib = address(volatile_mathlib);

        address volatile_template = contracts.volatile_template;
        if (volatile_template == address(0)) {
            volatile_template = address(
                new CatalystVaultVolatile{salt: bytes32(uint256(88648653197096078481732261810093446584084026526013867546544780279441951553797))}(address(factory), volatile_mathlib)
            );
        }
        contracts.volatile_template = address(volatile_template);

        address amplified_mathlib = contracts.amplified_mathlib;
        if (amplified_mathlib == address(0)) amplified_mathlib = address(new CatalystMathAmp{salt: bytes32(uint256(112267599032000726981556348618800808014156055146433051612035005254394394945477))}());
        contracts.amplified_mathlib = address(amplified_mathlib);

        address amplified_template = contracts.amplified_template;
        if (amplified_template == address(0)) {
            amplified_template = address(
                new CatalystVaultAmplified{salt: bytes32(uint256(77308261969657442296406024174680786851572270974085798458143326301183087901106))}(address(factory), amplified_mathlib)
            );
        }
        contracts.amplified_template = address(amplified_template);

        // Permit2 for router
        address permit2 = getOrDeployPermit2();

        // Get the wrapped token for router
        address wrappedGas = getGasToken();


        // Router
        CatalystRouter router = CatalystRouter(payable(contracts.router));
        if (address(router) == address(0)) {
            router = new CatalystRouter{salt: bytes32(0)}(RouterParameters({
                permit2: address(permit2),
                weth9: address(wrappedGas)
            }));
        }
        contracts.router = address(router);

        // Deploy Registry
        
        CatalystDescriber catalyst_describer = CatalystDescriber(contracts.describer);
        if (address(catalyst_describer) == address(0)) {
            catalyst_describer = new CatalystDescriber{salt: bytes32(0)}(vm.envAddress("CATALYST_ADDRESS"));
        }
        contracts.describer = address(catalyst_describer);

        {
            CatalystDescriberRegistry describer_registry = CatalystDescriberRegistry(contracts.describer_registry); 
            if (address(describer_registry) == address(0)) {
                describer_registry = new CatalystDescriberRegistry{salt: bytes32(0)}(vm.envAddress("CATALYST_ADDRESS"));
                fillDescriber = true;
            }
            contracts.describer_registry = address(describer_registry);
        }
    }

    function setupDescriber() internal {
        CatalystDescriber catalyst_describer = CatalystDescriber(contracts.describer);

        catalyst_describer.add_vault_factory(contracts.factory);
        catalyst_describer.add_whitelisted_template(contracts.volatile_template, 1);
        catalyst_describer.add_whitelisted_template(contracts.amplified_template, 1);
        whitelistAllCCIs(catalyst_describer);
    }


    function run() external {

        string memory pathRoot = vm.projectRoot();
        string memory pathToChainConfig = string.concat(pathRoot, "/script/config/config_chain.json");
        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory pathToTokenConfig = string.concat(pathRoot, "/script/config/config_tokens.json");

        // Get the chain config
        chain = vm.envString("CHAIN_NAME");
        string memory config_chain = vm.readFile(pathToChainConfig);
        chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", chain, ".chainIdentifier")), (bytes32));

        // Get the contracts
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".", chain)), (JsonContracts));

        // Get wrapped gas
        string memory config_token = vm.readFile(pathToTokenConfig);
        WGAS = abi.decode(config_token.parseRaw(string.concat(".", chain, ".", vm.envString("WGAS"))), (address));

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        deployAllContracts();

        // Fill registry
        // if (fillDescriber == true) {
        //     vm.startBroadcast(registryPrivateKey);
        //     setupDescriber();
        //     vm.stopBroadcast();
        // }

        vm.stopBroadcast();

        // Save json

        string memory obj = chain;

        vm.serializeAddress(obj, "amplified_mathlib", contracts.amplified_mathlib);
        vm.serializeAddress(obj, "amplified_template", contracts.amplified_template);
        vm.serializeAddress(obj, "describer", contracts.describer);
        vm.serializeAddress(obj, "describer_registry", contracts.describer_registry);
        vm.serializeAddress(obj, "factory", contracts.factory);
        vm.serializeAddress(obj, "permit2", contracts.permit2);
        vm.serializeAddress(obj, "router", contracts.router);
        vm.serializeAddress(obj, "volatile_mathlib", contracts.volatile_mathlib);
        string memory finalJson = vm.serializeAddress(obj, "volatile_template", contracts.volatile_template);
        
        // string memory finalJson = vm.serializeString(chain, "object", output);

        vm.writeJson(finalJson, pathToContractConfig, string.concat(".", chain));

    }
}

