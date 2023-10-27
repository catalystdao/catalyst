// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

abstract contract TestCrossChainInterfaceOnly is TestCommon, AVaultInterfaces {

    function test_error_receive_asset_no_permission(address caller) external {
        vm.assume(caller != address(CCI));
        vm.assume(caller != address(0));
        address[] memory vaults = getTestConfig();

        ICatalystV1Vault vault = ICatalystV1Vault(vaults[0]);
        vault.setConnection(bytes32(uint256(123)), abi.encodePacked(
            uint8(20),
            bytes32(0),
            abi.encode(address(vault))
        ), true);

        // Anyone not CCI cannot call.
        vm.prank(caller);
        vm.expectRevert();
        vault.receiveAsset(
            bytes32(uint256(123)),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(address(vault))
            ),
            0,
            address(caller),
            10**16,
            0,
            0,
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(address(vault))
            ),
            uint32(block.number)
        );

        // The CCI can call.
        vm.prank(address(CCI));
        vault.receiveAsset(
            bytes32(uint256(123)),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(address(vault))
            ),
            0,
            address(caller),
            10**16,
            0,
            0,
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(address(vault))
            ),
            uint32(block.number)
        );
    }

    function test_error_receive_liquidity_no_permission(address caller) external {
        vm.assume(caller != address(CCI));
        vm.assume(caller != address(0));
        address[] memory vaults = getTestConfig();

        ICatalystV1Vault vault = ICatalystV1Vault(vaults[0]);
        vault.setConnection(bytes32(uint256(123)), abi.encodePacked(
            uint8(20),
            bytes32(0),
            abi.encode(address(vault))
        ), true);

        // Anyone not CCI cannot call.
        vm.prank(caller);
        vm.expectRevert();
        vault.receiveLiquidity(
            bytes32(uint256(123)),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(address(vault))
            ),
            address(caller),
            10**16,
            0,
            0,
            0,
            uint32(block.number)
        );

        // The CCI can call.
        vm.prank(address(CCI));
        vault.receiveLiquidity(
            bytes32(uint256(123)),
            abi.encodePacked(
                uint8(20),
                bytes32(0),
                abi.encode(address(vault))
            ),
            address(caller),
            10**16,
            0,
            0,
            0,
            uint32(block.number)
        );
    }
}

