// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";

import { CatalystGARPInterface } from "../src/CatalystGARPInterface.sol";


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";

contract DeployInterfaces is Script {
    using stdJson for string;

    string pathToInterfacesConfig;

    error IncentivesIdNotFound();

    string[] incentive_versions;
    address[] incentive_addresses;

    string chain;
    bytes32 chainIdentifier;


    function getOrDeployGeneralisedIncentives(string memory version) internal returns(address incentive) {
        // Here is the map of id to version:
        // id == 0: Mock (POA)
        // id == 1: Wormhole

        if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("MOCK"))) {
            address signer = vm.envAddress("MOCK_SIGNER");

            uint256 pv_key = vm.envUint("MOCK_KEY");
            vm.startBroadcast(pv_key);

            incentive = address(new IncentivizedMockEscrow(chainIdentifier, signer));

            vm.stopBroadcast();
        } else if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("Wormhole"))) {
            uint256 pv_key = vm.envUint("WORMHOLE_KEY");
            vm.startBroadcast(pv_key);
            // TODO:
            vm.stopBroadcast();
        } else {
            revert IncentivesIdNotFound();
        }
    }

    function getOrDeployAllIncentives() internal {
        // read config_interfaces
        string memory config_interfaces = vm.readFile(pathToInterfacesConfig);

        string[] memory availableInterfaces = vm.parseJsonKeys(config_interfaces, string.concat(".", chain));

        for (uint256 i = 0; i < availableInterfaces.length; ++i) {
            string memory incentiveVersion = availableInterfaces[i];
            // Check if the incentives contract has already been deployed.
            address incentiveAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", chain, ".", incentiveVersion, ".interface")), (address));
            if (incentiveAddress == address(0)) continue;

            // otherwise we need to deploy it
            incentiveAddress = getOrDeployGeneralisedIncentives(incentiveVersion);

            // write the deployment
            string memory obj = chain;
            string memory finalJson = vm.serializeAddress(obj, "incentive", incentiveAddress);
            vm.writeJson(finalJson, pathToInterfacesConfig, string.concat(".", chain, ".", incentiveVersion));

            incentive_versions.push(incentiveVersion);
            incentive_addresses.push(incentiveAddress);
        }
    }

    function getOrDeployAllCCIs() internal {
        for (uint256 i = 0; i < incentive_versions.length; ++i) {
            string memory incentiveVersion = incentive_versions[i];
            address incentiveAddress = incentive_addresses[i];

            // otherwise we need to deploy it
            CatalystGARPInterface interfaceAddress = new CatalystGARPInterface(incentiveAddress);

            // Write
            vm.writeJson(Strings.toHexString(uint160(address(interfaceAddress)), 20), pathToInterfacesConfig, string.concat(".", chain, ".", incentiveVersion));
        }
    }

    function run() external {

        string memory pathRoot = vm.projectRoot();
        string memory pathToChainConfig = string.concat(pathRoot, "/script/config/config_chain.json");
        pathToInterfacesConfig = string.concat(pathRoot, "/script/config/config_interfaces.json");
        
        // Get the chain config
        chain = vm.envString("CHAIN_NAME");
        string memory config_chain = vm.readFile(pathToChainConfig);
        chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", chain, ".chainIdentifier")), (bytes32));


        getOrDeployAllIncentives();


        uint256 deployerPrivateKey = vm.envUint("CATALYST_INTERFACES_KEY");
        vm.startBroadcast(deployerPrivateKey);

        getOrDeployAllCCIs();

        vm.stopBroadcast();
    }
}

