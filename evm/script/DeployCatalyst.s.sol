// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";

import { BaseMultiChainDeployer } from "./BaseMultiChainDeployer.s.sol";
import { DeployContracts } from "./DeployContracts.s.soL";


contract DeployCatalyst is BaseMultiChainDeployer, DeployContracts {
    Chains[] chain_list;
    Chains[] chain_list_legacy;

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
        chain_list_legacy.push(Chains.ScrollSepolia);

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
        chain_list_legacy.push(Chains.MantleTestnet);
    }

    function deploy() iter_chains(chain_list) broadcast external {
        address admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);
    }
    function deploy_legacy() iter_chains(chain_list_legacy) broadcast external {
        address admin = vm.envAddress("CATALYST_ADDRESS");
        deployAllContracts(admin);
    }
}

