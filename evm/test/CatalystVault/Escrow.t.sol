// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

abstract contract TestEscrow is TestCommon, AVaultInterfaces {
    function test_escrow_ack() external {
        address[] memory vaults = getTestConfig();
        address bob = makeAddr("bob");

        bytes32 chainIdentifier = bytes32(0);
        setUpChains(chainIdentifier);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: chainIdentifier,
            toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])),
            toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault(vault).setConnection(chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])), true);

            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            uint256 amount = 251125521;

            assertEq(
                Token(fromToken).balanceOf(bob),
                0,
                "Bob has tokens?"
            );

            Token(fromToken).approve(vault, 2**256-1);

            uint32 bn = uint32(block.number);
            uint256 units = ICatalystV1Vault(vault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
                routeDescription,
                fromToken,
                0, 
                amount, 
                0, 
                address(bob),
                0,
                hex""
            );

            assertEq(
                Token(fromToken).balanceOf(bob),
                0,
                "Bob has more tokens than expected after send"
            );

            vm.prank(ICatalystV1Vault(vault)._chainInterface());
            ICatalystV1Vault(vault).onSendAssetSuccess(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            assertEq(
                Token(fromToken).balanceOf(bob),
                0,
                "Bob has more tokens than expected after send"
            );
        }
    }

    function test_escrow_timeout(uint256 swapPercentage) external {
        address[] memory vaults = getTestConfig();
        address bob = makeAddr("bob");

        bytes32 chainIdentifier = bytes32(0);
        setUpChains(chainIdentifier);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: chainIdentifier,
            toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])),
            toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault(vault).setConnection(chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])), true);

            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            uint256 amount = 251125521;

            assertEq(
                Token(fromToken).balanceOf(bob),
                0,
                "Bob has tokens?"
            );

            Token(fromToken).approve(vault, 2**256-1);

            uint32 bn = uint32(block.number);
            uint256 units = ICatalystV1Vault(vault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
                routeDescription,
                fromToken,
                0, 
                amount, 
                0, 
                address(bob),
                0,
                hex""
            );

            assertEq(
                Token(fromToken).balanceOf(bob),
                0,
                "Bob has more tokens than expected after send"
            );

            vm.prank(ICatalystV1Vault(vault)._chainInterface());
            ICatalystV1Vault(vault).onSendAssetFailure(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            assertEq(
                Token(fromToken).balanceOf(bob),
                amount,
                "Bob didn't get the return"
            );
        }
    }

    function test_only_one_response_ack() external {
        address[] memory vaults = getTestConfig();
        address bob = makeAddr("bob");

        bytes32 chainIdentifier = bytes32(0);
        setUpChains(chainIdentifier);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: chainIdentifier,
            toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])),
            toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault(vault).setConnection(chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])), true);

            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            uint256 amount = 251125521;

            Token(fromToken).approve(vault, 2**256-1);

            uint32 bn = uint32(block.number);
            uint256 units = ICatalystV1Vault(vault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
                routeDescription,
                fromToken,
                0, 
                amount, 
                0, 
                address(bob),
                0,
                hex""
            );

            vm.prank(ICatalystV1Vault(vault)._chainInterface());
            ICatalystV1Vault(vault).onSendAssetSuccess(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            vm.expectRevert();
            ICatalystV1Vault(vault).onSendAssetSuccess(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            vm.expectRevert();
            ICatalystV1Vault(vault).onSendAssetFailure(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );
        }
    }

    function test_only_one_response_timeout() external {
        address[] memory vaults = getTestConfig();
        address bob = makeAddr("bob");

        bytes32 chainIdentifier = bytes32(0);
        setUpChains(chainIdentifier);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: chainIdentifier,
            toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])),
            toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault(vault).setConnection(chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])), true);

            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            uint256 amount = 251125521;

            Token(fromToken).approve(vault, 2**256-1);

            uint32 bn = uint32(block.number);
            uint256 units = ICatalystV1Vault(vault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
                routeDescription,
                fromToken,
                0, 
                amount, 
                0, 
                address(bob),
                0,
                hex""
            );

            vm.prank(ICatalystV1Vault(vault)._chainInterface());
            ICatalystV1Vault(vault).onSendAssetFailure(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            vm.expectRevert();
            ICatalystV1Vault(vault).onSendAssetSuccess(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            vm.expectRevert();
            ICatalystV1Vault(vault).onSendAssetFailure(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );
        }
    }

    function test_escrow_impact_ack() external {
        address[] memory vaults = getTestConfig();
        address bob = makeAddr("bob");

        bytes32 chainIdentifier = bytes32(0);
        setUpChains(chainIdentifier);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: chainIdentifier,
            toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])),
            toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault(vault).setConnection(chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])), true);

            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            address toToken = ICatalystV1Vault(vault)._tokenIndexing(1);
            uint256 amount = 251125521;

            Token(fromToken).approve(vault, 2**256-1);

            uint256 U = 693147180559945344 / 2;  // Example value used to test if the swap is corrected.

            uint256 both1_12 = ICatalystV1Vault(vault).calcLocalSwap(fromToken, toToken, 10**18);
            uint256 both1_21 = ICatalystV1Vault(vault).calcLocalSwap(toToken, fromToken, 10**18);
            uint256 to1 = ICatalystV1Vault(vault).calcSendAsset(fromToken, 10**18);
            uint256 from1 = ICatalystV1Vault(vault).calcReceiveAsset(fromToken, U);

            uint32 bn = uint32(block.number);
            uint256 units = ICatalystV1Vault(vault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
                routeDescription,
                fromToken,
                0, 
                amount, 
                0, 
                address(bob),
                0,
                hex""
            );

            uint256 both2_12 = ICatalystV1Vault(vault).calcLocalSwap(fromToken, toToken, 10**18);
            uint256 both2_21 = ICatalystV1Vault(vault).calcLocalSwap(toToken, fromToken, 10**18);
            uint256 to2 = ICatalystV1Vault(vault).calcSendAsset(fromToken, 10**18);
            uint256 from2 = ICatalystV1Vault(vault).calcReceiveAsset(fromToken, U);

            assertGt(both1_12, both2_12, "Escrow not applied (or not set) on localswap(1)");
            assertEq(both1_21, both2_21, "Escrow mistakenly applied localswap(2)");
            assertGt(to1, to2, "Escrow not applied (or not set) on sendAsset");
            assertEq(from1, from2, "Escrow mistakenly applied sendAsset");

            vm.prank(ICatalystV1Vault(vault)._chainInterface());
            ICatalystV1Vault(vault).onSendAssetSuccess(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            uint256 both3_12 = ICatalystV1Vault(vault).calcLocalSwap(fromToken, toToken, 10**18);
            uint256 both3_21 = ICatalystV1Vault(vault).calcLocalSwap(toToken, fromToken, 10**18);
            uint256 to3 = ICatalystV1Vault(vault).calcSendAsset(fromToken, 10**18);
            uint256 from3 = ICatalystV1Vault(vault).calcReceiveAsset(fromToken, U);

            assertEq(both2_12, both3_12, "Priced updated, ack localswap?(1)");
            assertGt(both3_21, both1_21, "Signifiacnt issue with price update on localswap(2)");
            assertEq(to2, to3, "Priced updated, ack send?");
            if (from3 != from1) assertGt(from3, from1, "Issue with price update on receive");
        }
    }

    function test_escrow_impact_timeout() external {
        address[] memory vaults = getTestConfig();
        address bob = makeAddr("bob");

        bytes32 chainIdentifier = bytes32(0);
        setUpChains(chainIdentifier);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: chainIdentifier,
            toVault: abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])),
            toAccount: abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            ICatalystV1Vault(vault).setConnection(chainIdentifier, abi.encodePacked(uint8(20), bytes32(0), abi.encode(vaults[0])), true);

            address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
            address toToken = ICatalystV1Vault(vault)._tokenIndexing(1);
            uint256 amount = 251125521;

            Token(fromToken).approve(vault, 2**256-1);

            uint256 U = 693147180559945344 / 2;  // Example value used to test if the swap is corrected.

            uint256 both1_12 = ICatalystV1Vault(vault).calcLocalSwap(fromToken, toToken, 10**18);
            uint256 both1_21 = ICatalystV1Vault(vault).calcLocalSwap(toToken, fromToken, 10**18);
            uint256 to1 = ICatalystV1Vault(vault).calcSendAsset(fromToken, 10**18);
            uint256 from1 = ICatalystV1Vault(vault).calcReceiveAsset(fromToken, U);

            uint32 bn = uint32(block.number);
            uint256 units = ICatalystV1Vault(vault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
                routeDescription,
                fromToken,
                0, 
                amount, 
                0, 
                address(bob),
                0,
                hex""
            );

            uint256 both2_12 = ICatalystV1Vault(vault).calcLocalSwap(fromToken, toToken, 10**18);
            uint256 both2_21 = ICatalystV1Vault(vault).calcLocalSwap(toToken, fromToken, 10**18);
            uint256 to2 = ICatalystV1Vault(vault).calcSendAsset(fromToken, 10**18);
            uint256 from2 = ICatalystV1Vault(vault).calcReceiveAsset(fromToken, U);

            assertGt(both1_12, both2_12, "Escrow not applied (or not set) on localswap(1)");
            assertEq(both1_21, both2_21, "Escrow mistakenly applied localswap(2)");
            assertGt(to1, to2, "Escrow not applied (or not set) on sendAsset");
            assertEq(from1, from2, "Escrow mistakenly applied sendAsset");

            vm.prank(ICatalystV1Vault(vault)._chainInterface());
            ICatalystV1Vault(vault).onSendAssetFailure(
                chainIdentifier,
                abi.encodePacked(uint8(20), bytes32(0), abi.encode(address(bob))),
                units,
                amount,
                fromToken,
                bn
            );

            uint256 both3_12 = ICatalystV1Vault(vault).calcLocalSwap(fromToken, toToken, 10**18);
            uint256 both3_21 = ICatalystV1Vault(vault).calcLocalSwap(toToken, fromToken, 10**18);
            uint256 to3 = ICatalystV1Vault(vault).calcSendAsset(fromToken, 10**18);
            uint256 from3 = ICatalystV1Vault(vault).calcReceiveAsset(fromToken, U);

            assertEq(both1_12, both3_12, "Didn't revert back after timeout for localswap(1)");
            assertEq(both1_21, both3_21, "Price suddenly changed on localswap(2)");
            assertEq(to1, to3, "Didn't revert back after timeout for send");
            assertEq(from1, from3, "Price suddenly changed on receive");
        }
    }
}

