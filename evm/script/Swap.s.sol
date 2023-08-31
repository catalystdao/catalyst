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
import { ICatalystV1Structs } from "../src/interfaces/ICatalystV1VaultState.sol";


contract Swap is Script, IMessageEscrowStructs {

    function run() external {

        uint256 deployerPrivateKey = vm.envUint("CATALYST_DEPLOYER");
        vm.startBroadcast(deployerPrivateKey);

        address fromVault = address(0x25A8aB870e20A239e94570eD6024f996283F7418);
        address toVault = address(0x25A8aB870e20A239e94570eD6024f996283F7418);


        address WGAS = address(0xE67ABDA0D43f7AC8f37876bBF00D1DFadbB93aaa);
        Token(WGAS).approve(fromVault, 2**256-1);
        IWETH(WGAS).deposit{value: uint256(0.1*1e18)}();

        ICatalystV1Vault(fromVault).sendAsset{value: 2000000 * 10 gwei + 2000000 * 10 gwei}(
            ICatalystV1Structs.RouteDescription({
                chainIdentifier: bytes32(uint256(80001)),
                toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(toVault)),
                toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(this))),
                incentive: IncentiveDescription({
                    maxGasDelivery: 2000000,
                    maxGasAck: 2000000,
                    refundGasTo: address(this),
                    priceOfDeliveryGas: 10 gwei,
                    priceOfAckGas: 10 gwei,
                    targetDelta: 0 minutes
                })
            }),
            WGAS,
            0,
            uint256(0.1*1e18),
            0,
            address(this),
            hex""
        );

        vm.stopBroadcast();

    }

    receive() external payable {
    }
}

