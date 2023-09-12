// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";

import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";

import { BaseMultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

import { CatalystDescriber } from "../src/registry/CatalystDescriber.sol";

// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";

contract DeployInterfaces is BaseMultiChainDeployer {
    using stdJson for string;

    string config_interfaces;
    string config_chain;
    
    error IncentivesIdNotFound();

    string incentiveVersion;

    mapping(address => bytes32) interfaceSalt;

    constructor() {
        interfaceSalt[0x000000641AC10b4e000fe361F2149E2a531061c5] = bytes32(0xd2c66ec619a687874ed1cbc01390b279ae3822887485f5ee26b1fa083dcaf1f9);
    }

    function deployGeneralisedIncentives(string memory version, bytes32 chainIdentifier) internal returns(address incentive) {
        // Here is the map of id to version:
        // id == 0: Mock (POA)
        // id == 1: Wormhole

        if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("MOCK"))) {
            address signer = vm.envAddress("MOCK_SIGNER");

            vm.stopBroadcast();
            uint256 pv_key = vm.envUint("INCENTIVE_DEPLOYER");
            vm.startBroadcast(pv_key);

            incentive = address(new IncentivizedMockEscrow(chainIdentifier, signer, 0));

            vm.stopBroadcast();
            vm.startBroadcast(pk);

        } else if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("Wormhole"))) {
            revert IncentivesIdNotFound();
        } else {
            revert IncentivesIdNotFound();
        }
    }

    modifier forEachInterface() {
        string[] memory availableInterfaces = vm.parseJsonKeys(config_interfaces, string.concat(".", rpc[chain]));
        for (uint256 i = 0; i < availableInterfaces.length; ++i) {
            incentiveVersion = availableInterfaces[i];

            _;
        }
    }

    modifier load_config() {
        string memory pathRoot = vm.projectRoot();
        string memory pathToChainConfig = string.concat(pathRoot, "/script/config/config_chain.json");
        string memory pathToInterfacesConfig = string.concat(pathRoot, "/script/config/config_interfaces.json");
        config_interfaces = vm.readFile(pathToInterfacesConfig);
        
        // Get the chain config
        config_chain = vm.readFile(pathToChainConfig);

        _;
    }

    function deployBaseInterfaces(bytes32 chainIdentifier) forEachInterface() internal {
        // Get the address of the incentives contract.
        address incentiveAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".incentive")), (address));
        if (incentiveAddress.codehash != bytes32(0)) {
            console.logAddress(incentiveAddress);
            return;
        }
        address newlyDeployedIncentiveAddress = deployGeneralisedIncentives(incentiveVersion, chainIdentifier);
        require(newlyDeployedIncentiveAddress == incentiveAddress, "Newly deployed incentive address isn't expected address");
    }

    function deployCCI(address admin) forEachInterface() internal {
        // Get the address of the incentives contract.
        address interfaceAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".interface")), (address));
        if (interfaceAddress.codehash != bytes32(0)) {
            console.logAddress(interfaceAddress);
            return;
        }
        address incentiveAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".incentive")), (address));

        address newlyDeployedInterfaceAddress = address(new CatalystChainInterface{salt: interfaceSalt[incentiveAddress]}(incentiveAddress, admin));
        require(newlyDeployedInterfaceAddress == interfaceAddress, "Newly deployed interface address isn't expected address");
    }

    function whitelistCCI(address catalyst_describer) forEachInterface() internal {
        CatalystDescriber describer = CatalystDescriber(catalyst_describer);

        address interfaceAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".interface")), (address));

        if (describer.get_num_whitelisted_ccis() == 0) {
            describer.add_whitelisted_cci(interfaceAddress);
        }
    }

    function deploy() load_config iter_chains(chain_list) broadcast external {
        address admin = vm.envAddress("CATALYST_ADDRESS");
        bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", rpc[chain], ".chainIdentifier")), (bytes32));
        deployBaseInterfaces(chainIdentifier);

        deployCCI(admin);

        whitelistCCI(0x8950BAe1ADc61D28300009b4C2CfddfE5f55cb52);
    }

    function deploy_legacy() load_config iter_chains(chain_list_legacy) broadcast external {
        address admin = vm.envAddress("CATALYST_ADDRESS");
        bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", rpc[chain], ".chainIdentifier")), (bytes32));
        deployBaseInterfaces(chainIdentifier);

        deployCCI(admin);

        whitelistCCI(0x8950BAe1ADc61D28300009b4C2CfddfE5f55cb52);
    }
}

