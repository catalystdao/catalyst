// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";

import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";

import { BaseMultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

import { CatalystDescriber } from "../src/registry/CatalystDescriber.sol";

// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IncentivizedWormholeEscrow } from "GeneralisedIncentives/src/apps/wormhole/IncentivizedWormholeEscrow.sol";


contract DeployInterfaces is BaseMultiChainDeployer {
    using stdJson for string;

    string config_interfaces;
    string config_chain;
    
    error IncentivesIdNotFound();

    string incentiveVersion;

    mapping(address => bytes32) interfaceSalt;

    bytes32 constant KECCACK_OF_NOTHING = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470;

    mapping(Chains => address) wormholeBridge;

    constructor() {
        interfaceSalt[0x00000001a9818a7807998dbc243b05F2B3CfF6f4] = bytes32(0xfb5d7080c10f7c4a95069b43b0ba06d246a8a9a6f2f3fbbbfde47a6f6eb2d3ca);

        interfaceSalt[0x000000ED80503e3A7EA614FFB5507FD52584a1f2] = bytes32(0x526041c1059f5a0db3ed779269c367ee5637a3427bf73365ec3b94bafddad14c);

        wormholeBridge[Chains.Sepolia] = 0x4a8bc80Ed5a4067f1CCf107057b8270E0cC11A78;

        wormholeBridge[Chains.Mumbai] = 0x0CBE91CF822c73C2315FB05100C2F714765d5c20;
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
            vm.stopBroadcast();
            uint256 pv_key = vm.envUint("WORMHOLE_DEPLOYER");
            vm.startBroadcast(pv_key);

            incentive = address(new IncentivizedWormholeEscrow(wormholeBridge[chain]));

            vm.stopBroadcast();
            vm.startBroadcast(pk);
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

    function deployBaseIncentive(bytes32 chainIdentifier) forEachInterface() internal {
        // Get the address of the incentives contract.
        address incentiveAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".incentive")), (address));
        console.log("inc", incentiveVersion);
        if (incentiveAddress.codehash != bytes32(0)) {
            console.logAddress(incentiveAddress);
            return;
        }
        address newlyDeployedIncentiveAddress = deployGeneralisedIncentives(incentiveVersion, chainIdentifier);
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

        console.logAddress(newlyDeployedInterfaceAddress);

        require(newlyDeployedInterfaceAddress == interfaceAddress, "Newly deployed interface address isn't expected address");
    }

    function whitelistCCI(address catalyst_describer) forEachInterface() internal {
        CatalystDescriber describer = CatalystDescriber(catalyst_describer);

        address interfaceAddress = abi.decode(config_interfaces.parseRaw(string.concat(".", rpc[chain], ".", incentiveVersion, ".interface")), (address));

        bool foundCCI = false;
        address[] memory ccis = describer.get_whitelisted_CCI();
        for (uint256 i = 0; i < ccis.length; ++i) {
            address ci = ccis[i];
            if (ci == interfaceAddress) {
                foundCCI = true;
                break;
            }
        }

        if (foundCCI == false) {
            describer.add_whitelisted_cci(interfaceAddress);
        }
    }

    function unwhitelistCCI() load_config iter_chains(chain_list) broadcast external {
        CatalystDescriber describer = CatalystDescriber(0xfB933A070D9a1D43CF973714e35bed7e4a5A0545);
        address[] memory whitelistedCCI = describer.get_whitelisted_CCI();

        bool found = false;
        for (uint256 i = 0; i < whitelistedCCI.length; ++i) {
            address cci = whitelistedCCI[i];

            if (cci == 0x0000000CC613E3Da01da44B438B6916849529128) {
                found = true;
            }
            if (cci == 0x0000000c5ebB5b2bE933e98dFE9A441b58A2820E) {
                describer.remove_whitelisted_cci(cci, i);
            }
            console.logAddress(cci);
        }

        if (found == false) {
            describer.add_whitelisted_cci(0x0000000CC613E3Da01da44B438B6916849529128);
        }
    }

    function unwhitelistCCI_legacy() load_config iter_chains(chain_list_legacy) broadcast external {
        CatalystDescriber describer = CatalystDescriber(0xfB933A070D9a1D43CF973714e35bed7e4a5A0545);
        address[] memory whitelistedCCI = describer.get_whitelisted_CCI();

        bool found = false;
        for (uint256 i = 0; i < whitelistedCCI.length; ++i) {
            address cci = whitelistedCCI[i];

            if (cci == 0x0000000CC613E3Da01da44B438B6916849529128) {
                found = true;
            }
            if (cci == 0x0000000c5ebB5b2bE933e98dFE9A441b58A2820E) {
                describer.remove_whitelisted_cci(cci, i);
            }
            console.logAddress(cci);
        }

        if (found == false) {
            describer.add_whitelisted_cci(0x0000000CC613E3Da01da44B438B6916849529128);
        }
    }
    
    function _deploy() internal {
        address admin = address(0x0000007aAAC54131e031b3C0D6557723f9365A5B);
        bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", rpc[chain], ".chainIdentifier")), (bytes32));

        fund(vm.envAddress("INCENTIVE_DEPLOYER_ADDRESS"), 0.05*10**18);

        deployBaseIncentive(chainIdentifier);

        deployCCI(admin);

        whitelistCCI(0x7C52D11EDe2AAA85562dE3D485592F40E0C87615);
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

            bytes32 chainIdentifier = abi.decode(config_chain.parseRaw(string.concat(".", rpc[remoteChain], ".chainIdentifier")), (bytes32));
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

