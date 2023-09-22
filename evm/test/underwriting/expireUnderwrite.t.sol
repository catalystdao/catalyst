// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

import { ICatalystV1Vault } from "../../src/ICatalystV1Vault.sol";
import { CatalystChainInterface } from "../../src/CatalystChainInterface.sol";
import { Token } from "../mocks/token.sol";

contract TestExpireUnderwrite is TestCommon {
    address vault1;
    address vault2;
    
    function setUp() virtual override public {
        super.setUp();
        vault1 = simpleVault(1);
        vault2 = simpleVault(1);

        // Setup
        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);
    }
    
    function test_error_expire_non_existing_underwrite() external {
        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        bytes32 identifier = CCI.getUnderwriteIdentifier(
            vault2,
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        vm.expectRevert(
            abi.encodeWithSignature("UnderwriteDoesNotExist(bytes32)", (
                identifier
            ))
        );
        CCI.expireUnderwrite(
            vault2,
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );
    }

    function test_underwrite_expire_storage_reset(address expirer) external {
        vm.assume(expirer != address(0));
        uint256 maxUnderwritingDuration = CCI.maxUnderwritingDuration();

        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        bytes32 identifier = CCI.underwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        // check that storage has been set.
        (, address refundTo, ) = CCI.underwritingStorage(identifier);
        assertEq(
            refundTo,
            address(this)
        );

        vm.warp(block.timestamp + maxUnderwritingDuration * 2);

        vm.prank(expirer);
        CCI.expireUnderwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        (, refundTo, ) = CCI.underwritingStorage(identifier);
        assertEq(
            refundTo,
            address(0)
        );
    }

    function test_allow_underwrite_to_always_expire() external {
        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        CCI.underwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        CCI.expireUnderwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );
    }

    function test_error_require_expiry(address expirer) external {
        vm.assume(expirer != address(0));
        vm.assume(expirer != address(this));

        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        bytes32 identifier = CCI.underwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        (, ,uint80 expiry) = CCI.underwritingStorage(identifier);


        vm.prank(expirer);
        vm.expectRevert(
            abi.encodeWithSignature("UnderwriteNotExpired(uint256)", (
                expiry-block.timestamp
            ))
        );
        CCI.expireUnderwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );
    }

    function test_empty_cci_after_expiry() external {
        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        bytes32 identifier = CCI.underwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        CCI.expireUnderwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            hex"0000"
        );

        assertEq(
            Token(token).balanceOf(address(CCI)),
            0,
            "CCI balance is not 0"
        );
    }
}

