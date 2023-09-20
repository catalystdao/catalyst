// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.17;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";

import { BaseMultiChainDeployer} from "../BaseMultiChainDeployer.s.sol";

import { MockApplication } from "./MockApplication.sol";

import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";

import { IncentivizedWormholeEscrow } from "GeneralisedIncentives/src/apps/wormhole/IncentivizedWormholeEscrow.sol";

contract DeployMock is BaseMultiChainDeployer, IMessageEscrowStructs {
    function run() broadcast external {
        address mock = address(new MockApplication{salt: bytes32(0)}(0x000000ED80503e3A7EA614FFB5507FD52584a1f2));

        require(mock == 0x19dC9f1C9c49B431103ba80a28c206C4a65Dc80c, "non");

        MockApplication(mock).setRemoteEscrowImplementation(bytes32(uint256(5)), abi.encode(mock));
        MockApplication(mock).setRemoteEscrowImplementation(bytes32(uint256(10002)), abi.encode(mock));
    }


    function sendMessageWithMock() broadcast external {
        IncentivizedWormholeEscrow(0x000000ED80503e3A7EA614FFB5507FD52584a1f2);
        address mock = 0x19dC9f1C9c49B431103ba80a28c206C4a65Dc80c;

        MockApplication(mock).escrowMessage{value: 200000 * 10 gwei * 2}(
            bytes32(uint256(5)),
            abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(mock))),
            hex"0123456789abcdeffedcba9876543210",
            IncentiveDescription({
                maxGasDelivery: 200000,
                maxGasAck: 200000,
                refundGasTo: address(0x0000007aAAC54131e031b3C0D6557723f9365A5B),
                priceOfDeliveryGas: 10 gwei,
                priceOfAckGas: 10 gwei,
                targetDelta: 0 minutes
            })
        );
    }
}

