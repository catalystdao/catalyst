// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Permit2 } from "../lib/permit2/src/Permit2.sol";
import { Strings } from "@openzeppelin/contracts/utils/Strings.sol";
import { Token } from "../test/mocks/token.sol";
import { IWETH } from "./IWETH.sol";
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";

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
import { CatalystGARPInterface } from "../src/CatalystGARPInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";


contract Swap is Script, IMessageEscrowStructs {

    function run() external {

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        address fromVault = address(0xB9A11Db41ac2A1e7A8F4BdFd353999f43bD79704);
        address toVault = address(0xB9A11Db41ac2A1e7A8F4BdFd353999f43bD79704);

        Token(address(0x0000005eff5E63a9B4C4505Af65F701354d0Bef7)).approve(fromVault, 2**256-1);

        ICatalystV1Vault(fromVault).sendAsset{value: 1 ether}(
            bytes32(uint256(2)),
            abi.encodePacked(uint8(20), bytes32(0), abi.encode(toVault)),
            abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(this))),
            address(0x0000005eff5E63a9B4C4505Af65F701354d0Bef7),
            0,
            uint256(0.01*1e18),
            0,
            address(this),
            IncentiveDescription({
                maxGasDelivery: 2000000,
                maxGasAck: 2000000,
                refundGasTo: address(this),
                priceOfDeliveryGas: 10 gwei,
                priceOfAckGas: 10 gwei,
                targetDelta: 0 minutes
            })
        );


        vm.stopBroadcast();

    }
}

