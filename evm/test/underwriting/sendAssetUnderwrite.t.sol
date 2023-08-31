// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

import { ICatalystV1Structs } from "../../src/interfaces/ICatalystV1VaultState.sol";
import { ICatalystV1Vault } from "../../src/ICatalystV1Vault.sol";
import { CatalystGARPInterface } from "../../src/CatalystGARPInterface.sol";
import { Token } from "../mocks/token.sol";

import { ICatalystReceiver } from "../../src/interfaces/IOnCatalyst.sol";

contract TestSendAssetUnderwrite is TestCommon, ICatalystReceiver {
    address vault1;
    address vault2;

    bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0xfee0eec191fa4f)));
    
    function setUp() virtual override public {
        super.setUp();
        vault1 = simpleVault(1);
        vault2 = simpleVault(1);

        // Setup
        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);
    }

    function test_send_asset_underwrite(address refundTo, address toAccount) external {
        vm.assume(refundTo != address(0));
        vm.assume(toAccount != address(0));
        vm.assume(toAccount != refundTo);  // makes it really hard to debug
        // execute the swap.
        address token1 = ICatalystV1Vault(vault1)._tokenIndexing(0);

        Token(token1).approve(vault1, 2**256-1);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(vault2),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE
        });


        vm.recordLogs();
        uint256 units = ICatalystV1Vault(vault1).sendAssetUnderwrite{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            token1,
            0, 
            uint256(1e17), 
            0,
            toAccount,
            0,
            hex""
        );
        Vm.Log[] memory entries = vm.getRecordedLogs();

        (bytes32 destinationIdentifier, bytes memory recipitent, bytes memory messageWithContext) = abi.decode(entries[1].data, (bytes32, bytes, bytes));


        address token2 = ICatalystV1Vault(vault2)._tokenIndexing(0);


        Token(token2).approve(address(CCI), 2**256-1);
        
        bytes32 underwriteIdentifier = CCI.underwrite(
            refundTo, // non-zero address
            vault2,  // -- Swap information
            token2,
            units,
            0,
            toAccount,
            uint256(1e17),
            0,
            hex"0000"
        );

        (uint256 numTokens, , ) = CCI.underwritingStorage(underwriteIdentifier);

        assertEq(
            Token(token2).balanceOf(address(CCI)),
            numTokens * (
                CCI.UNDERWRITING_UNFULFILLED_FEE()
            )/CCI.UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR(),
            "CCI balance incorrect"
        );

        // Then let the package arrive.
        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        GARP.processMessage(_metadata, toExecuteMessage, FEE_RECIPITANT);

        assertEq(
            Token(token2).balanceOf(address(CCI)),
            0,
            "CCI balance not 0"
        );

        assertEq(
            Token(token2).balanceOf(refundTo),
            numTokens + numTokens * (
                CCI.UNDERWRITING_UNFULFILLED_FEE()
            )/CCI.UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR(),
            "refundTo balance not expected"
        );
    }
    
    bytes on_catalyst_call_data;

    function onCatalystCall(uint256 purchasedTokens, bytes calldata data) external {
        on_catalyst_call_data = data;
    }
}

