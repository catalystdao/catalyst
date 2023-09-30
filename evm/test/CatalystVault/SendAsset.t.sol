// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
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

abstract contract TestSendAsset is TestCommon, AVaultInterfaces {

    uint256 private constant MARGIN_NUM = 1;
    uint256 private constant MARGIN_DENOM = 1e12;

    /// forge-config: default.fuzz.runs = 1000
    function test_sendAsset(bytes32 channelId, uint32 swapSizePercentage, address toAccount) external virtual {
        vm.assume(toAccount != address(0));
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;
        uint8 fromAssetIndex = 0;
        uint8 toAssetIndex = 0;

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];


        setConnection(fromVault, toVault, channelId, channelId);

        uint256 initial_invariant = invariant(vaults);

        address fromToken = ICatalystV1Vault(fromVault)._tokenIndexing(fromAssetIndex);

        uint256 amount = getLargestSwap(fromVault, toVault, fromToken, ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex)) * uint256(swapSizePercentage) / (2**32 - 1);

        Token(fromToken).approve(fromVault, amount);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: channelId,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE
        });

        uint256 units = ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            toAssetIndex, 
            uint256(amount), 
            uint256(amount)/2, 
            toAccount,
            0,
            hex""
        );

        uint256 after_invariant = invariant(vaults);

        if (after_invariant + units < initial_invariant) {
            assertGt(
                initial_invariant * MARGIN_NUM / MARGIN_DENOM,
                initial_invariant - after_invariant,
                "Swap error beyond margin found"
            );
        }
    }

    function test_error_failback_address_0(bytes32 channelId, uint56 amount, address toAccount) external virtual {
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;
        uint8 fromAssetIndex = 0;
        uint8 toAssetIndex = 0;

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];

        setConnection(fromVault, toVault, channelId, channelId);

        address fromToken = ICatalystV1Vault(fromVault)._tokenIndexing(fromAssetIndex);
        Token(fromToken).approve(fromVault, amount);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: channelId,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE
        });

        vm.expectRevert();

        ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            toAssetIndex, 
            uint256(amount), 
            uint256(amount)/2, 
            address(0),
            0,
            hex""
        );
    }

    function test_sendAsset_fee(uint32 swapSizePercentage, uint56 fee) external virtual {
        address toAccount = address(this);
        bytes32 channelId = bytes32(uint256(123));
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;
        uint8 fromAssetIndex = 0;
        uint8 toAssetIndex = 0;

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];

        setConnection(fromVault, toVault, channelId, channelId);

        uint256 initial_invariant = invariant(vaults);

        address fromToken = ICatalystV1Vault(fromVault)._tokenIndexing(fromAssetIndex);

        uint256 amount = getLargestSwap(fromVault, toVault, fromToken, ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex)) * uint256(swapSizePercentage) / (2**32 - 1);

        Token(fromToken).approve(fromVault, amount);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: channelId,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE
        });


        uint256 ss = vm.snapshot();

        uint256 unitsLowerAmount = ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            toAssetIndex, 
            uint256(amount) - uint256(amount) * uint256(fee) / 10**18, 
            uint256(amount)/2, 
            toAccount,
            0,
            hex""
        );

        vm.revertTo(ss);

        ICatalystV1Vault(fromVault).setVaultFee(fee);

        uint256 unitsWithFee = ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            toAssetIndex, 
            uint256(amount), 
            uint256(amount)/2, 
            toAccount,
            0,
            hex""
        );


        assertEq(
            unitsLowerAmount,
            unitsWithFee,
            "Fee not correctly taken"
        );
    }
}

