// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";
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
import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";
import { ICatalystV1Structs } from "../src/interfaces/ICatalystV1VaultState.sol";

import { IncentivizedWormholeEscrow } from "GeneralisedIncentives/src/apps/wormhole/IncentivizedWormholeEscrow.sol";

contract Swap is Script, IMessageEscrowStructs {

    function getChainIdentifierWormhole() external {
        address wormhole_incentive = 0x000000ED80503e3A7EA614FFB5507FD52584a1f2;

        console.logUint(IncentivizedWormholeEscrow(wormhole_incentive).chainId());
    }

    function swap(uint256 n) public {

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        address fromVault = address(0x8eEfc0F0E47994dcF7542ff080b8970cD4CF09EC);
        address toVault = address(0x8eEfc0F0E47994dcF7542ff080b8970cD4CF09EC);

        // mantle
        address WGAS = address(0x9c3C9283D3e44854697Cd22D3Faa240Cfb032889);

        uint256 amount = 0.0001 * 1e18;

        Token(WGAS).approve(fromVault, 2**256-1);
        IWETH(WGAS).deposit{value: amount}();

        for (uint256 i = 0; i < n; ++i) {
            ICatalystV1Vault(fromVault).sendAsset{value: 2000000 * 10 gwei + 2000000 * 10 gwei}(
                ICatalystV1Structs.RouteDescription({
                    chainIdentifier: bytes32(uint256(10002)),
                    toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(toVault)),
                    toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(0x0000007aAAC54131e031b3C0D6557723f9365A5B))),
                    incentive: IncentiveDescription({
                        maxGasDelivery: 2000000,
                        maxGasAck: 2000000,
                        refundGasTo: address(0x0000007aAAC54131e031b3C0D6557723f9365A5B),
                        priceOfDeliveryGas: 10 gwei,
                        priceOfAckGas: 10 gwei,
                        targetDelta: 0 minutes
                    })
                }),
                WGAS,
                0,
                amount/n,
                0,
                address(0x0000007aAAC54131e031b3C0D6557723f9365A5B),
                hex""
            );
        }

        vm.stopBroadcast();

    }

    function run() external {
        swap(1);
    }

    receive() external payable {
    }
}

