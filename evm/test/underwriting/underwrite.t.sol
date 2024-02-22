// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

import { ICatalystV1Vault } from "../../src/ICatalystV1Vault.sol";
import { CatalystChainInterface } from "../../src/CatalystChainInterface.sol";
import { Token } from "../mocks/token.sol";

import { ICatalystReceiver } from "../../src/interfaces/IOnCatalyst.sol";

contract TestUnderwrite is TestCommon, ICatalystReceiver {
    address vault1;
    address vault2;
    
    function setUp() virtual override public {
        super.setUp();
        vault1 = simpleVault(1);
        vault2 = simpleVault(1);

        // Setup
        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);
    }

    function test_underwrite_storage(address sendTo) external {
        vm.assume(sendTo != vault1);
        vm.assume(sendTo != vault2);
        vm.assume(sendTo != address(0));
        vm.assume(sendTo != 0x7FA9385bE102ac3EAc297483Dd6233D62b3e1496);
        uint256 maxUnderwritingDuration = CCI.maxUnderwritingDuration();

        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);

        bytes32 identifier = CCI.underwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            sendTo,
            0,
            hex"0000"
        );

        (uint256 tokensStorage, address refundToStorage, uint96 expiryStorage) = CCI.underwritingStorage(identifier);

        assertEq(address(this), refundToStorage, "RefundTo isn't the same as storage");
        assertEq(expiryStorage, block.timestamp + maxUnderwritingDuration, "Expiry storage not correctly set");

        // Check the balance of CCI.

        assertEq(
            Token(token).balanceOf(address(CCI)),
             tokensStorage * (
                CCI.UNDERWRITING_COLLATERAL()
            )/CCI.UNDERWRITING_COLLATERAL_DENOMINATOR(),
            "Tokens storage not correctly set"
        );

        // check the balance of sendTo

        assertEq(
            Token(token).balanceOf(sendTo),
            tokensStorage,
            "token sent not correct"
        );
    }

    function test_underwrite_sub_call(bytes calldata base_cdata) external {
        address token = ICatalystV1Vault(vault2)._tokenIndexing(0);

        Token(token).approve(address(CCI), 2**256-1);
        bytes memory encoded_calldata = abi.encodePacked(address(this), base_cdata);

        CCI.underwrite(
            vault2,  // -- Swap information
            token,
            1e17,
            0,
            address(this),
            0,
            abi.encodePacked(
                uint16(encoded_calldata.length),
                encoded_calldata
            )
        );

        assertEq(
            on_catalyst_call_data,
            base_cdata,
            "Calldata does not match"
        );
    }
    
    bytes on_catalyst_call_data;

    function onCatalystCall(uint256 /* purchasedTokens */, bytes calldata data) external {
        on_catalyst_call_data = data;
    }
}

