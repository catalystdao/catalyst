// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
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
import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";

contract BaseMultiChainDeployer is Script {
    Chains[] chain_list;

    enum Stage {
        test,
        prod
    }

    enum Chains {
        Mumbai,
        Sepolia,
        BaseGoerli,
        ArbitrumGoerli,
        ScrollSepolia,
        OptimismGoerli,
        TaikoEldfell,
        OPBNBTestnet,
        BSCTestnet,
        MantleTestnet
    }

    mapping(Chains => string) public rpc;
    mapping(Chains => string) public wrapped_gas;

    uint256 pk;

    constructor() {
        rpc[Chains.Mumbai] = "mumbai";
        wrapped_gas[Chains.Mumbai] = "WMATIC";
        chain_list.push(Chains.Mumbai);

        rpc[Chains.Sepolia] = "sepolia";
        wrapped_gas[Chains.Sepolia] = "WETH10";
        chain_list.push(Chains.Sepolia);

        rpc[Chains.BaseGoerli] = "basegoerli";
        wrapped_gas[Chains.BaseGoerli] = "WETH";
        chain_list.push(Chains.BaseGoerli);

        rpc[Chains.ArbitrumGoerli] = "arbitrumgoerli";
        wrapped_gas[Chains.ArbitrumGoerli] = "WETH";
        chain_list.push(Chains.ArbitrumGoerli);

        rpc[Chains.ScrollSepolia] = "scrollsepolia";
        wrapped_gas[Chains.ScrollSepolia] = "WETH";
        chain_list.push(Chains.ScrollSepolia);

        rpc[Chains.OptimismGoerli] = "optimismgoerli";
        wrapped_gas[Chains.OptimismGoerli] = "WETH";
        chain_list.push(Chains.OptimismGoerli);

        rpc[Chains.TaikoEldfell] = "taikoeldfell";
        wrapped_gas[Chains.TaikoEldfell] = "WETH";
        chain_list.push(Chains.TaikoEldfell);

        rpc[Chains.OPBNBTestnet] = "opbnbtestnet";
        wrapped_gas[Chains.OPBNBTestnet] = "WBNB";
        chain_list.push(Chains.OPBNBTestnet);

        rpc[Chains.BSCTestnet] = "bsctestnet";
        wrapped_gas[Chains.BSCTestnet] = "WBNB";
        chain_list.push(Chains.BSCTestnet);

        rpc[Chains.MantleTestnet] = "mantletestnet";
        wrapped_gas[Chains.MantleTestnet] = "WETH";
        chain_list.push(Chains.MantleTestnet);
    }


    function selectFork(Chains chain) internal {
        console.log("vm.envString(rpc[chain])");
        vm.createSelectFork(vm.envString(rpc[chain]));
    }


    modifier iter_chains(Chains[] memory chains) {
        for (uint256 chainIndex = 0; chainIndex < chains.length; ++chainIndex) {

            Chains chain = chains[chainIndex];

            selectFork(chain);

            _;
        }
    }

    modifier broadcast() {
        pk = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(pk);

        _;

        vm.stopBroadcast();
    }
}

