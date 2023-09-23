// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";

import { BaseMultiChainDeployer} from "./BaseMultiChainDeployer.s.sol";

import { CatalystRouter } from "../src/router/CatalystRouter.sol";
import { RouterParameters } from "../src/router/base/RouterImmutables.sol";

contract DeployRouter is BaseMultiChainDeployer {
    using stdJson for string;

    address expectedRouterAddress = address(0x000000672d633a391D8ba094B798D4B098fB32EA);
    address expectedPermit2Address = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);

    string config_token;

    modifier load_config() {
        string memory pathRoot = vm.projectRoot();
        string memory pathToTokenConfig = string.concat(pathRoot, "/script/config/config_tokens.json");
        config_token = vm.readFile(pathToTokenConfig);

        _;
    }

    function deployPermit2() public {
        if (expectedPermit2Address.codehash != bytes32(0)) return;

        // We cannot deploy permit2 because of the way remappings work. As a result, we
        // can only check if it exists and if it doesn't manually deploy it later.
        console.log("! -- Deploy Permit2 to", rpc[chain], "-- !");
    }

    function deployRouter() public {
        vm.stopBroadcast();
        vm.startBroadcast(vm.envUint("ROUTER_DEPLOYER"));

        if (expectedRouterAddress.codehash != bytes32(0)) return;

        CatalystRouter router = new CatalystRouter(RouterParameters({
            permit2: expectedPermit2Address,
            weth9: abi.decode(config_token.parseRaw(string.concat(".", rpc[chain], ".", wrapped_gas[chain])), (address))
        }));

        require(
            address(router) == expectedRouterAddress,
            "unexpected Deployment Address router"
        );

        vm.stopBroadcast();
        vm.startBroadcast(pk);
    }
    
    function _deploy() internal {
        fund(vm.envAddress("ROUTER_DEPLOYER_ADDRESS"), 0.01*10**18);

        deployPermit2();
        
        deployRouter();
    }

    function deploy() load_config iter_chains(chain_list) broadcast external {
        _deploy();
    }

    function deploy_legacy() load_config iter_chains(chain_list_legacy) broadcast external {
        _deploy();
    }
}

