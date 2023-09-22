// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

import { ICatalystV1Vault } from "../../src/ICatalystV1Vault.sol";
import { CatalystChainInterface } from "../../src/CatalystChainInterface.sol";
import { Token } from "../mocks/token.sol";

import { ICatalystReceiver } from "../../src/interfaces/IOnCatalyst.sol";

contract TestUnderwriteAndCheckConnection is TestCommon {
    address vault1;
    address vault2;
    
    function setUp() virtual override public {
        super.setUp();
        // Setup
        vault1 = simpleVault(1);
        vault2 = simpleVault(1);
    }

    function test_error_underwriteAndCheckConnection() external {
        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        vm.expectRevert(
            abi.encodeWithSignature("NoVaultConnection()")
        );
        CCI.underwriteAndCheckConnection(
            DESTINATION_IDENTIFIER,
            abi.encodePacked(
                uint8(20), bytes32(0), abi.encode(vault1)
            ),
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );
    }

    function test_underwriteAndCheckConnection() external {
        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);

        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        CCI.underwriteAndCheckConnection(
            DESTINATION_IDENTIFIER,
            abi.encodePacked(
                uint8(20), bytes32(0), abi.encode(vault1)
            ),
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );
    }
}

