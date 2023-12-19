// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import { Strings } from "openzeppelin-contracts/contracts/utils/Strings.sol";
import { Token } from "../test/mocks/token.sol";
import { IWETH } from "./IWETH.sol";
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";


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

        address fromVault = address(0x40939B7a3A6F3523C38458b2E9b0181583D1f18E);
        address toVault = address(0x40939B7a3A6F3523C38458b2E9b0181583D1f18E);

        // mantle
        address WGAS = ICatalystV1Vault(fromVault)._tokenIndexing(0);

        uint256 amount = 0.0001 * 1e18;

        Token(WGAS).approve(fromVault, 2**256-1);
        IWETH(WGAS).deposit{value: amount}();

        for (uint256 i = 0; i < n; ++i) {
            ICatalystV1Vault(fromVault).sendAsset{value: 0.1 ether}(
                ICatalystV1Structs.RouteDescription({
                    chainIdentifier: bytes32(uint256(44963396551096171397160558726282299286140034337171331191205222918429706551296)),
                    toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(toVault)),
                    toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(0x0000007aAAC54131e031b3C0D6557723f9365A5B))),
                    incentive: IncentiveDescription({
                        maxGasDelivery: 2000000,
                        maxGasAck: 2000000,
                        refundGasTo: address(0x0000007aAAC54131e031b3C0D6557723f9365A5B),
                        priceOfDeliveryGas: 5 gwei,
                        priceOfAckGas: 5 gwei,
                        targetDelta: 0 minutes
                    })
                }),
                WGAS,
                0,
                amount/n,
                2**256-1,
                address(0x0000007aAAC54131e031b3C0D6557723f9365A5B),
                0,
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

