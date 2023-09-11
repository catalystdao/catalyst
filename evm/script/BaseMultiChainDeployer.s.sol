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

    function selectFork(Chains chain) internal {
        console.log(vm.envString(rpc[chain]));
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

