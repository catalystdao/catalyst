// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";

import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";

import { BaseMultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IncentivizedWormholeEscrow } from "GeneralisedIncentives/src/apps/wormhole/IncentivizedWormholeEscrow.sol";

import { IncentivizedPolymerEscrow } from "GeneralisedIncentives/src/apps/polymer/IncentivizedPolymerEscrow.sol";

import { JsonContracts } from "./DeployContracts.s.sol";

contract DeployInterfaces is BaseMultiChainDeployer {
    using stdJson for string;

    string config_interfaces;
    string config_chain;
    
    error IncentivesIdNotFound();

    string incentiveVersion;

    JsonContracts contracts;

    mapping(address => bytes32) interfaceSalt;

    bytes32 constant KECCACK_OF_NOTHING = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470;

    mapping(Chains => address) wormholeBridge;

    mapping(Chains => address) polymerContract;

    constructor() {
        interfaceSalt[0x00000001a9818a7807998dbc243b05F2B3CfF6f4] = bytes32(uint256(1));

        interfaceSalt[0x000000ED80503e3A7EA614FFB5507FD52584a1f2] = bytes32(uint256(1));

        wormholeBridge[Chains.Sepolia] = 0x4a8bc80Ed5a4067f1CCf107057b8270E0cC11A78;
        wormholeBridge[Chains.Mumbai] = 0x0CBE91CF822c73C2315FB05100C2F714765d5c20;

        polymerContract[Chains.BaseSepolia] = 0xfcef85E0F0Afd1Acd73fAF1648266DF923d4521d;
        polymerContract[Chains.OptimismSepolia] = 0x3001b73254EB715799EB93E8413EdCE4721090Ab;
    }

    function deployGeneralisedIncentives(string memory version) internal returns(address incentive) {
        // Here is the map of id to version:
        // id == 0: Mock (POA)
        // id == 1: Wormhole
        if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("MOCK"))) {
            bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", version, ".", rpc[chain], ".", rpc[chain])), (bytes32));

            address signer = vm.envAddress("MOCK_SIGNER");

            vm.stopBroadcast();
            uint256 pv_key = vm.envUint("INCENTIVE_DEPLOYER");
            vm.startBroadcast(pv_key);

            incentive = address(new IncentivizedMockEscrow(vm.envAddress("CATALYST_ADDRESS"), chainIdentifier, signer, 0));

            vm.stopBroadcast();
            vm.startBroadcast(pk);

        } else if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("Wormhole"))) {
            vm.stopBroadcast();
            uint256 pv_key = vm.envUint("WORMHOLE_DEPLOYER");
            vm.startBroadcast(pv_key);

            require(wormholeBridge[chain] != address(0), "Contract to send messages to cannot be 0.");
            incentive = address(new IncentivizedWormholeEscrow(vm.envAddress("CATALYST_ADDRESS"), wormholeBridge[chain]));

            vm.stopBroadcast();
            vm.startBroadcast(pk);
        } else if (keccak256(abi.encodePacked(version)) == keccak256(abi.encodePacked("Polymer"))) {
            vm.stopBroadcast();
            uint256 pv_key = vm.envUint("POLYMER_DEPLOYER");
            vm.startBroadcast(pv_key);

            require(polymerContract[chain] != address(0), "Contract to send messages to cannot be 0.");
            incentive = address(new IncentivizedPolymerEscrow(vm.envAddress("CATALYST_ADDRESS"), polymerContract[chain]));

            vm.stopBroadcast();
            vm.startBroadcast(pk);
        } else {
            revert IncentivesIdNotFound();
        }
    }

    modifier forEachInterface() {
        if (!vm.keyExists(config_interfaces, string.concat(".", rpc[chain]))) return;
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

        string memory pathToContractConfig = string.concat(pathRoot, "/script/config/config_contracts.json");
        string memory config_contract = vm.readFile(pathToContractConfig);
        contracts = abi.decode(config_contract.parseRaw(string.concat(".contracts")), (JsonContracts));
        
        
        // Get the chain config
        config_chain = vm.readFile(pathToChainConfig);

        _;
    }

    function deployBaseIncentive() forEachInterface() internal {
        // Get the address of the incentives contract.
        address incentiveAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".incentive")), (address));
        console.log("inc", incentiveVersion);
        if (incentiveAddress.codehash != bytes32(0)) {
            console.logAddress(incentiveAddress);
            return;
        }
        address newlyDeployedIncentiveAddress = deployGeneralisedIncentives(incentiveVersion);
        console.log("Deploying new base incentive");
        console.logAddress(newlyDeployedIncentiveAddress);
        require(newlyDeployedIncentiveAddress == incentiveAddress, "Newly deployed incentive address isn't expected address");
    }

    function deployCCI(address admin) forEachInterface() internal {
        // Get the address of the incentives contract.
        address interfaceAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".interface")), (address));
        console.log("cci", incentiveVersion);
        if (interfaceAddress.codehash != bytes32(0)) {
            console.logAddress(interfaceAddress);
            return;
        }
        address incentiveAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".incentive")), (address));

        bytes32 salt = interfaceSalt[incentiveAddress];

        address newlyDeployedInterfaceAddress = address(
            new CatalystChainInterface{salt: salt}(incentiveAddress, admin)
        );

        console.log("Deploying new base interface");
        console.logAddress(newlyDeployedInterfaceAddress);

        require(newlyDeployedInterfaceAddress == interfaceAddress, "Newly deployed interface address isn't expected address");
    }

    
    function _deploy() internal {
        address admin = address(0x0000007aAAC54131e031b3C0D6557723f9365A5B);

        // fund(vm.envAddress("INCENTIVE_DEPLOYER_ADDRESS"), 0.05*10**18);

        deployBaseIncentive();

        deployCCI(admin);

        // get describer
    }

    function deploy() load_config iter_chains(chain_list) broadcast external {
        _deploy();
    }

    function deploy_legacy() load_config iter_chains(chain_list_legacy) broadcast external {
        _deploy();
    }

    function _connect_cci() forEachInterface internal {
        Chains[] memory all_chains = new Chains[](chain_list.length + chain_list_legacy.length);
        uint256 i = 0;
        for (i = 0; i < chain_list.length; ++i) {
            all_chains[i] = chain_list[i];
        }
        for (uint256 j = 0; j < chain_list_legacy.length; ++j) {
            all_chains[i+j] = chain_list_legacy[j];
        }


        CatalystChainInterface cci = CatalystChainInterface(abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".interface")), (address)));

        for (i = 0; i < all_chains.length; ++i) {
            Chains remoteChain = all_chains[i];
            if (chain == remoteChain) continue;
            if (
                !vm.keyExists(config_interfaces, string.concat(".", rpc[remoteChain], ".", incentiveVersion))
            ) continue;

            bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", incentiveVersion, ".", rpc[chain], ".",  rpc[remoteChain])), (bytes32));
            // check if a connection has already been set.

            if (keccak256(cci.chainIdentifierToDestinationAddress(chainIdentifier)) != KECCACK_OF_NOTHING) {
                console2.log(
                "skipping",
                rpc[chain],
                rpc[remoteChain]
            );
                continue;
            }

            address remoteInterface = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[remoteChain], ".", incentiveVersion, ".interface")), (address));
            address remoteIncentive = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[remoteChain], ".", incentiveVersion, ".incentive")), (address));

            console2.log(
                "connecting",
                rpc[chain],
                rpc[remoteChain]
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

    function connect_cci() load_config iter_chains(chain_list) broadcast external {
        _connect_cci();
    }

    function connect_cci_legacy() load_config iter_chains(chain_list_legacy) broadcast external {
        _connect_cci();
    }
}

