// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

interface TF {
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool);
}

abstract contract TestSendLiquidity is TestCommon, AVaultInterfaces {

    uint256 private constant MARGIN_NUM = 1;
    uint256 private constant MARGIN_DENOM = 1e12;

    /// forge-config: default.fuzz.runs = 1000
    function test_sendLiquidity(bytes32 channelId, uint32 swapSizePercentage, address toAccount) external virtual {
        vm.assume(toAccount != address(0));
        vm.assume(swapSizePercentage != type(uint32).max);
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];


        setConnection(fromVault, toVault, channelId, channelId);

        uint256 amount = Token(fromVault).balanceOf(address(this)) * swapSizePercentage / (2**32 - 1);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: channelId,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        ICatalystV1Vault(fromVault).sendLiquidity{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            amount,
            [uint256(0), uint256(0)],
            address(this),
            hex""
        );
    }

     function test_error_sendLiquidity_fallback_0(bytes32 channelId, uint32 swapSizePercentage) external virtual {
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];


        setConnection(fromVault, toVault, channelId, channelId);

        uint256 amount = Token(fromVault).balanceOf(address(this)) * swapSizePercentage / (2**32 - 1);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: channelId,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(address(this)),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        vm.expectRevert();

        ICatalystV1Vault(fromVault).sendLiquidity{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            amount,
            [uint256(0), uint256(0)],
            address(0),
            hex""
        );
    }

}