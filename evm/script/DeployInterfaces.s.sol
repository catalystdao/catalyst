// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";

import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";

import { MultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

contract DeployInterfaces is MultiChainDeployer {
    using stdJson for string;

    address private admin;

    string config_chain;
    string config_bridge;
    string config_interfaces;

    string pathToBridgeConfig;
    string pathToInterfacesConfig;
    
    error IncentivesIdNotFound();

    string bridgeVersion;

    mapping(address => bytes32) escrowSalt;

    bytes32 constant KECCACK_OF_NOTHING = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470;

    mapping(string => mapping(string => address)) escrowContract;

    constructor() {
        escrowSalt[0x00000001a9818a7807998dbc243b05F2B3CfF6f4] = bytes32(uint256(1));

        escrowSalt[0x000000ED80503e3A7EA614FFB5507FD52584a1f2] = bytes32(uint256(1));
    }

    modifier load_config() {
        string memory pathRoot = vm.projectRoot();

        pathToInterfacesConfig = string.concat(pathRoot, "/script/config/config_interfaces.json");
        config_interfaces = vm.readFile(pathToInterfacesConfig);
        
        string memory pathToChainConfig = string.concat(pathRoot, "/lib/catalyst-channel-lists/src/config/chains.json");
        config_chain = vm.readFile(pathToChainConfig);

        pathToBridgeConfig = string.concat(pathRoot, "/lib/GeneralisedIncentives/script/bridge_contracts.json");
        config_bridge = vm.readFile(pathToBridgeConfig);

        // Get the bridges available
        string[] memory availableBridges = vm.parseJsonKeys(config_bridge, "$");

        // For each bridge, decode their escrows' contract for each chain.
        for (uint256 i = 0; i < availableBridges.length; ++i) {
            string memory bridge = availableBridges[i];
            // Get the chains this bridge support.
            string[] memory availableBridgesChains = vm.parseJsonKeys(config_bridge, string.concat(".", bridge));
            for (uint256 j = 0; j < availableBridgesChains.length; ++j) {
                string memory chain = availableBridgesChains[j];
                // decode the address
                address _escrowAddress = vm.parseJsonAddress(config_bridge, string.concat(".", bridge, ".", chain, ".escrow"));
                escrowContract[bridge][chain] = _escrowAddress;
            }
        }

        _;
    }

    modifier forEachInterface(string[] memory bridges) {
        for (uint256 i = 0; i < bridges.length; ++i) {
            bridgeVersion = bridges[i];
            // Write the escrow address.
            address escrow = escrowContract[bridgeVersion][currentChainKey];
            // Check if there is a valid chain key.
            if (!vm.keyExists(config_interfaces, string.concat(".", bridgeVersion, ".", currentChainKey))) continue;
            vm.writeJson(
                vm.toString(escrow),
                pathToInterfacesConfig,
                string.concat(".", bridgeVersion, ".", currentChainKey, ".escrow")
            );

            _;
        }
    }

    function deployCCI(string[] memory bridges) forEachInterface(bridges) internal returns(address interfaceAddress) {
        // Get the address of the incentives contract.
        address escrowAddress = escrowContract[bridgeVersion][currentChainKey];
        // If the incentive contract is 0, skip.
        if (escrowAddress == address(0)) {
            console.log("Incentive address 0");
            return address(0);
        }

        bytes32 salt = escrowSalt[escrowAddress];

        // Get the expected deployment address
        address expectedInterfaceAddress = _getAddress(
            abi.encodePacked(
                type(CatalystChainInterface).creationCode,
                abi.encode(escrowAddress, admin)
            ),
            salt
        );

        if (expectedInterfaceAddress.codehash != bytes32(0)) return interfaceAddress = expectedInterfaceAddress;

        interfaceAddress = address(
            new CatalystChainInterface{salt: salt}(escrowAddress, admin)
        );

        // Write the interface address
        vm.writeJson(
            vm.toString(interfaceAddress),
            pathToInterfacesConfig,
            string.concat(".", bridgeVersion, ".", currentChainKey, ".interface")
        );
    }

    // get the computed address before the contract DeployWithCreate2 deployed using Bytecode of contract
    function _getAddress(bytes memory bytecode, bytes32 _salt) internal pure returns (address) {
        bytes32 create2Hash = keccak256(
            abi.encodePacked(
                bytes1(0xff), address(0x4e59b44847b379578588920cA78FbF26c0B4956C), _salt, keccak256(bytecode)
            )
        );
        return address(uint160(uint(create2Hash)));
    }

    
    function _deploy(string[] memory bridges) internal {
        admin = vm.addr(pk);

        deployCCI(bridges);
    }

    function deploy(string[] memory bridges, string[] memory chains) load_config iter_chains_string(chains) broadcast external {
        _deploy(bridges);
    }

    function deployAll() load_config iter_chains(chain_list) broadcast external {
        // Get the bridges available
        string[] memory availableBridges = vm.parseJsonKeys(config_bridge, "$");

        _deploy(availableBridges);
    }

    function deployAllLegacy() load_config iter_chains(chain_list_legacy) broadcast external {
        // Get the bridges available
        string[] memory availableBridges = vm.parseJsonKeys(config_bridge, "$");

        _deploy(availableBridges);
    }

    function _connect_cci(string[] memory bridges, string[] memory counterpartChains) forEachInterface(bridges) internal {

        CatalystChainInterface cci = CatalystChainInterface(abi.decode(config_interfaces.parseRaw(string.concat(".", bridgeVersion, ".", currentChainKey, ".interface")), (address)));

        for (uint256 i = 0; i < counterpartChains.length; ++i) {
            string memory remoteChain = counterpartChains[i];
            if (keccak256(abi.encodePacked(currentChainKey)) == keccak256(abi.encodePacked(remoteChain))) continue;
            if (
                !vm.keyExists(config_interfaces, string.concat(".", bridgeVersion, ".", remoteChain))
            ) continue;

            // Check if there exists a remote deployment.
            if (
                !vm.keyExists(config_interfaces, string.concat(".", bridgeVersion, ".", remoteChain))
            ) {
                console2.log(
                    "no-deployment",
                    bridgeVersion,
                    remoteChain
                );
                continue;
            }

            // Check if the chain identifier expists.
            if (
                !vm.keyExists(config_chain, string.concat(".", bridgeVersion, ".", currentChainKey, ".",  remoteChain))
            ) {
                console2.log(
                    "no-chainidentifier",
                    currentChainKey,
                    remoteChain
                );
                continue;
            }

            bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", bridgeVersion, ".", currentChainKey, ".",  remoteChain)), (bytes32));

            // check if a connection has already been set.
            if (keccak256(cci.chainIdentifierToDestinationAddress(chainIdentifier)) != KECCACK_OF_NOTHING) {
                console2.log(
                "skipping",
                currentChainKey,
                remoteChain
            );
                continue;
            }

            address remoteInterface = abi.decode(config_interfaces.parseRaw(string.concat(".", bridgeVersion, ".", remoteChain, ".interface")), (address));
            address remoteIncentive = abi.decode(config_interfaces.parseRaw(string.concat(".", bridgeVersion, ".", remoteChain, ".escrow")), (address));

            console2.log(
                "connecting",
                currentChainKey,
                remoteChain
            );

            cci.connectNewChain(
                chainIdentifier, 
                abi.encodePacked(
                    uint8(20), 
                    bytes32(0), 
                    abi.encode(remoteInterface)
                ),
                abi.encode(remoteIncentive)
            );
        }
    }

    function connectCCI(string[] calldata bridges, string[] calldata chains) load_config iter_chains_string(chains) broadcast external {
        _connect_cci(bridges, chains);
    }

    function connectCCI(string[] calldata bridges, string[] calldata localChains, string[] calldata remoteChains) load_config iter_chains_string(localChains) broadcast external {
        _connect_cci(bridges, remoteChains);
    }
}

