// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

import { ICatalystV1Structs } from "../../src/interfaces/ICatalystV1VaultState.sol";
import { ICatalystV1Vault } from "../../src/ICatalystV1Vault.sol";
import { CatalystChainInterface } from "../../src/CatalystChainInterface.sol";
import { Token } from "../mocks/token.sol";

import { ICatalystReceiver } from "../../src/interfaces/IOnCatalyst.sol";

contract TestUnderwriteNoConnection is TestCommon {
    address vault1;
    address vault2;

    bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0xfee0eec191fa4f)));

    event SwapFailed(bytes1 error);
    event SendAssetFailure(
        bytes32 channelId,
        bytes toAccount,
        uint256 units,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    );
    event Transfer(address indexed from, address indexed to, uint256 amount);
    
    function setUp() virtual override public {
        super.setUp();
        // Setup
        vault1 = simpleVault(1);
        vault2 = simpleVault(1);

        // set the connection
        setUpChains(DESTINATION_IDENTIFIER);

        // only connect vault 1 to vault2
        ICatalystV1Vault(vault1).setConnection(
            DESTINATION_IDENTIFIER,
            convertEVMTo65(vault2),
            true
        );
    }

    function test_error_underwrite_no_connection(address refundTo, address toAccount) external {
        vm.assume(refundTo != address(0));
        vm.assume(toAccount != address(0));
        vm.assume(toAccount != refundTo);  // makes it really hard to debug
        vm.assume(toAccount != vault1);
        vm.assume(toAccount != address(CCI));
        vm.assume(toAccount != address(this));
        address token1 = ICatalystV1Vault(vault1)._tokenIndexing(0);

        Token(token1).approve(vault1, 2**256-1);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(vault2),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE
        });

        vm.recordLogs();
        uint256 units = ICatalystV1Vault(vault1).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
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

        (, , bytes memory messageWithContext) = abi.decode(entries[1].data, (bytes32, bytes, bytes));

        // Then let the package arrive.
        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        vm.recordLogs();
        GARP.processMessage(_metadata, toExecuteMessage, FEE_RECIPITANT);
        entries = vm.getRecordedLogs();

        address token2 = ICatalystV1Vault(vault2)._tokenIndexing(0);

        assertEq(
            Token(token2).balanceOf(address(CCI)),
            0,
            "CCI balance not 0"
        );

        // Lets execute the message on the source chain and check for the swap revert message.
        (,, messageWithContext) = abi.decode(entries[1].data, (bytes32, bytes, bytes));
        (_metadata, toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);
    
        // we need to check that 
        vm.expectEmit();
        emit SwapFailed(0x13);
        vm.expectEmit();
        emit Transfer(vault1, toAccount, uint256(1e17));
        vm.expectEmit();
        emit SendAssetFailure(DESTINATION_IDENTIFIER, convertEVMTo65(toAccount), units, uint256(1e17), address(token1), 1);


        GARP.processMessage(_metadata, toExecuteMessage, FEE_RECIPITANT);
    }
}

