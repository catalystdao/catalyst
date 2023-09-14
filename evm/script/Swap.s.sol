// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

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
import { CatalystChainInterface } from "../src/CatalystChainInterface.sol";
/// Catalyst Templates
import { CatalystVaultVolatile } from "../src/CatalystVaultVolatile.sol";
import { CatalystVaultAmplified } from "../src/CatalystVaultAmplified.sol";


// Generalised Incentives
import { IncentivizedMockEscrow } from "GeneralisedIncentives/src/apps/mock/IncentivizedMockEscrow.sol";
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";
import { ICatalystV1Structs } from "../src/interfaces/ICatalystV1VaultState.sol";


contract Swap is Script, IMessageEscrowStructs {

    function swap(uint256 n) public {

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        address fromVault = address(0xB0D751e5E8a337515f7faa528ef807727998c401);
        address toVault = address(0xB0D751e5E8a337515f7faa528ef807727998c401);

        // mantle
        address WGAS = address(0x4200000000000000000000000000000000000006);

        uint256 amount = 0.0001 * 1e18;

        Token(WGAS).approve(fromVault, 2**256-1);
        IWETH(WGAS).deposit{value: amount}();

        for (uint256 i = 0; i < n; ++i) {
            ICatalystV1Vault(fromVault).sendAsset{value: 2000000 * 10 gwei + 2000000 * 10 gwei}(
                ICatalystV1Structs.RouteDescription({
                    chainIdentifier: bytes32(uint256(84531)),
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

