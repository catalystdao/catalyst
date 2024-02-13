// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";

contract BaseMultiChainDeployer is Script {
    enum Stage {
        test,
        prod
    }

    enum Chains {
        Sepolia,
        ArbitrumSepolia,
        OptimismSepolia
    }

    mapping(Chains => string) public rpc;
    mapping(Chains => string) public wrapped_gas;


    Chains[] chain_list;
    Chains[] chain_list_legacy;

    Chains chain;

    constructor() {
        rpc[Chains.Sepolia] = "sepolia";
        wrapped_gas[Chains.Sepolia] = "WETH10";
        chain_list.push(Chains.Sepolia);

        rpc[Chains.ArbitrumSepolia] = "arbitrumsepolia";
        wrapped_gas[Chains.ArbitrumSepolia] = "WETH";
        chain_list.push(Chains.ArbitrumSepolia);

        rpc[Chains.OptimismSepolia] = "optimismsepolia";
        wrapped_gas[Chains.OptimismSepolia] = "WETH";
        chain_list.push(Chains.OptimismSepolia);
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

