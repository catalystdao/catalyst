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
    address crosschaininterface;
    address describer;
    address describer_registry;
    address factory;
    address permit2;
    address router;
    address volatile_mathlib;
    address volatile_template;
}

contract DeployInterfaces is Script {
    using stdJson for string;

    error IncentivesIdNotFound();

    string config_interfaces;

    string chain;
    bytes32 chainIdentifier;


    function getOrDeployGeneralisedIncentives(string memory version) internal returns(address incentive) {
        // Here is the map of id to version:
        // id == 0: Mock (POA)
        // id == 1: Wormhole

        if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("MOCK"))) {
            string memory json_structure = string.concat(
                ".",
                chain,
                ".",
                version,
                "."
                "incentive"
            );
            incentive = abi.decode(config_interfaces.parseRaw(json_structure), (address));
            if (incentive != address(0)) return incentive;

            // Load the signer
            json_structure = string.concat(
                ".",
                chain,
                ".",
                version,
                "."
                "signer"
            );
            address signer = abi.decode(config_interfaces.parseRaw(json_structure), (address));

            incentive = address(new IncentivizedMockEscrow(chainIdentifier, signer));
        } else if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("Wormhole"))) {
            // TODO:
        } else {
            revert IncentivesIdNotFound();
        }
    }

    function deployCCI(string memory version) internal {
        address incentive = getOrDeployGeneralisedIncentives(version);

        CatalystGARPInterface cci = new CatalystGARPInterface(incentive);
    }

    function deployAllCCIs() internal {
        string memory json_structure = string.concat(
            ".",
            chain,
            ".",
            "available"
        );
        string[] memory available_versions = abi.decode(
            config_interfaces.parseRaw(json_structure), (string[])
        );

        for (uint256 i = 0; i < available_versions.length; ++i) {
            string memory key = available_versions[i];
            deployCCI(key);
        }
    }


    function run() external {
        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_KEY");

        
        chain = vm.envString("CHAIN_NAME");
        string memory config_chain = vm.readFile("./config/config_chain.json");
        chainIdentifier = bytes32(vm.parseJsonUint(config_chain, chain));

        vm.startBroadcast(deployerPrivateKey);

        deployAllCCIs();

        vm.stopBroadcast();
    }
}

