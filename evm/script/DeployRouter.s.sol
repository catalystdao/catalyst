// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

import { MultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

import { CatalystRouter } from "../src/router/CatalystRouter.sol";
import { RouterParameters } from "../src/router/base/RouterImmutables.sol";

contract DeployRouter is MultiChainDeployer {
    using stdJson for string;

    address expectedRouterAddress = address(0x00000029e6005863Bb2E1686a17C4ae0D1723669);
    address expectedPermit2Address = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);

    string config_token;

    modifier load_config() {
        string memory pathRoot = vm.projectRoot();
        string memory pathToTokenConfig = string.concat(pathRoot, "/script/config/config_tokens.json");
        config_token = vm.readFile(pathToTokenConfig);

        _;
    }

    function deployPermit2() view public {
        if (expectedPermit2Address.codehash != bytes32(0)) return;

        // We cannot deploy permit2 because of the way remappings work. As a result, we
        // can only check if it exists and if it doesn't manually deploy it later.
        console.log("! -- Deploy Permit2 to", currentChainKey, "-- !");
    }

    function deployRouter() public {
        vm.stopBroadcast();
        vm.startBroadcast(vm.envUint("ROUTER_DEPLOYER"));

        if (expectedRouterAddress.codehash != bytes32(0)) return;

        address gasToken = abi.decode(config_token.parseRaw(string.concat(".", currentChainKey, ".", wrappedGas[currentChainKey])), (address));


        require(gasToken != address(0), "Gas token cannot be address0");

        CatalystRouter router = new CatalystRouter(RouterParameters({
            permit2: expectedPermit2Address,
            weth9: gasToken
        }));

        require(
            address(router) == expectedRouterAddress,
            "unexpected Deployment Address router"
        );

        vm.stopBroadcast();
        vm.startBroadcast(pk);
    }
    
    function _deploy() internal {
        deployPermit2();
        
        deployRouter();
    }

    function deploy(string[] calldata chains) load_config iter_chains_string(chains) broadcast external {
        _deploy();
    }

    function deploy_legacy() load_config iter_chains(chain_list_legacy) broadcast external {
        _deploy();
    }
}

