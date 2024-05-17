// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";

// Import MultiChainDeployer from garp
import { BaseMultiChainDeployer } from "GeneralisedIncentives/script/BaseMultiChainDeployer.s.sol";

contract MultiChainDeployer is BaseMultiChainDeployer {

    mapping(string => string) wrappedGas;

    constructor() BaseMultiChainDeployer() {
        wrappedGas[chainKey[Chains.Sepolia]] = "WETH10";

        wrappedGas[chainKey[Chains.BaseSepolia]] = "WETH";

        wrappedGas[chainKey[Chains.ArbitrumSepolia]] = "WETH";

        wrappedGas[chainKey[Chains.OptimismSepolia]] = "WETH";

        wrappedGas[chainKey[Chains.BlastTestnet]] = "WETH";
    }
}

