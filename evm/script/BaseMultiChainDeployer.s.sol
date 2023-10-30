// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";

contract BaseMultiChainDeployer is Script {
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
        // TaikoEldfell,
        OPBNBTestnet,
        BSCTestnet,
        MantleTestnet,
        OmniTestnet,
        INEVMDevnet
    }

    mapping(Chains => string) public rpc;
    mapping(Chains => string) public wrapped_gas;


    Chains[] chain_list;
    Chains[] chain_list_legacy;

    Chains chain;

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

        // rpc[Chains.TaikoEldfell] = "taikoeldfell";
        // wrapped_gas[Chains.TaikoEldfell] = "WETH";
        // chain_list.push(Chains.TaikoEldfell);

        rpc[Chains.OPBNBTestnet] = "opbnbtestnet";
        wrapped_gas[Chains.OPBNBTestnet] = "WBNB";
        chain_list.push(Chains.OPBNBTestnet);

        rpc[Chains.BSCTestnet] = "bsctestnet";
        wrapped_gas[Chains.BSCTestnet] = "WBNB";
        chain_list.push(Chains.BSCTestnet);

        rpc[Chains.MantleTestnet] = "mantletestnet";
        wrapped_gas[Chains.MantleTestnet] = "WMNT";
        chain_list_legacy.push(Chains.MantleTestnet);

        rpc[Chains.OmniTestnet] = "omnitestnet";
        wrapped_gas[Chains.OmniTestnet] = "WOMNI";
        chain_list_legacy.push(Chains.OmniTestnet);

        rpc[Chains.INEVMDevnet] = "inevmdevnet";
        wrapped_gas[Chains.INEVMDevnet] = "WINJ";
        chain_list.push(Chains.INEVMDevnet);
    }

    uint256 pk;

    function selectFork(Chains chain_) internal {
        console.log(vm.envString(rpc[chain_]));
        vm.createSelectFork(vm.envString(rpc[chain_]));
    }


    modifier iter_chains(Chains[] memory chains) {
        for (uint256 chainIndex = 0; chainIndex < chains.length; ++chainIndex) {

            chain = chains[chainIndex];

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

    function fund(address toFund, uint256 amount) internal {
        if (toFund.balance >= amount) return;

        payable(toFund).transfer(amount);
    }
}

