// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import { ICatalystV1Vault } from "src/ICatalystV1Vault.sol";
import { ICatalystV1VaultEvents } from "src/interfaces/ICatalystV1VaultEvents.sol";
import { VaultNotConnected } from "src/interfaces/ICatalystV1VaultErrors.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import { Token } from "test/mocks/token.sol";
import { AVaultInterfaces } from "test/CatalystVault/AVaultInterfaces.t.sol";
import { TestInvariant } from "test/CatalystVault/Invariant.t.sol";


abstract contract TestVaultConnections is TestCommon, AVaultInterfaces {

    bytes32 constant MOCK_CHANNEL_ID = bytes32(uint256(0xabc));

    // NOTE: The following addresses are encoded in the 65-bytes form.
    bytes constant MOCK_REMOTE_VAULT = hex"140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000CA7A1AB5CA7A1AB5CA7A1AB5CA7A1AB5CA7A1AB5";
    bytes constant MOCK_TO_ACCOUNT = hex"1400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001";


    function test_ConnectVaults() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);

        // Verify the connection is unset
        bool connectionState = vault._vaultConnection(MOCK_CHANNEL_ID, MOCK_REMOTE_VAULT);
        assert(!connectionState);

        // Verify the SetConnection event
        vm.expectEmit();
        emit ICatalystV1VaultEvents.SetConnection (
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );



        // Tested action: set the vault connection
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );



        // Verify the connection is set
        connectionState = vault._vaultConnection(MOCK_CHANNEL_ID, MOCK_REMOTE_VAULT);
        assert(connectionState);
    }


    function test_DisconnectVaults() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);
        
        // Setup the connection
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );
        bool connectionState = vault._vaultConnection(MOCK_CHANNEL_ID, MOCK_REMOTE_VAULT);
        assert(connectionState);

        // Verify the SetConnection event
        vm.expectEmit();
        emit ICatalystV1VaultEvents.SetConnection (
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            false
        );



        // Tested action: unset the vault connection
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            false
        );



        // Verify the connection is unset
        connectionState = vault._vaultConnection(MOCK_CHANNEL_ID, MOCK_REMOTE_VAULT);
        assert(!connectionState);
    }


    function test_InvalidAuth(address notSetupMaster) external {

        vm.assume(notSetupMaster != address(this));

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);



        // Tested action: call 'setConnection' by an unauthorized account
        vm.prank(notSetupMaster);
        vm.expectRevert();
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );

    }


    function test_InvalidRemoteVaultAddress() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);



        // Tested action: set the vault connection with an invalid remote address
        vm.expectRevert();
        vault.setConnection(
            MOCK_CHANNEL_ID,
            hex"abcd",    // invalid address: not 65 bytes
            true
        );

    }


    function test_ConnectVaultsAfterFinishSetup() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);

        // Finish setup
        vault.finishSetup();


        
        // Tested action: set connection after finish setup
        vm.expectRevert();
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );

    }


    function test_NoConnectionSendAsset() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);

        address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);

        uint256 amount = 100000;

        Token(fromToken).approve(address(vault), amount);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: MOCK_CHANNEL_ID,
            toVault: MOCK_REMOTE_VAULT,
            toAccount: MOCK_TO_ACCOUNT,
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });



        // Tested action: sendAsset without a valid connection
        vm.expectRevert(VaultNotConnected.selector);
        vault.sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            0,
            amount,
            0,
            address(this),
            0,
            hex""
        );



        // Verify sendAsset works once the connection is set
        vault.setConnection(    // Set vault connection
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );
        CCI.connectNewChain(    // Connect the generalised incentives with a mock remote endpoint
            MOCK_CHANNEL_ID,
            convertEVMTo65(address(CCI)),
            abi.encode(address(GARP))
        );
        
        vault.sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            0,
            amount,
            0,
            address(this),
            0,
            hex""
        );

    }


    function test_NoConnectionReceiveAsset() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);



        // Tested action: receiveAsset without a valid connection
        vm.prank(address(CCI));
        vm.expectRevert(VaultNotConnected.selector);
        vault.receiveAsset(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            0,
            address(this),
            1000000,
            0,
            0,
            hex"",
            0
        );



        // Verify receiveAsset works once the connection is set
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );

        vm.prank(address(CCI));
        vault.receiveAsset(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            0,
            address(this),
            1000000,
            0,
            0,
            hex"",
            0
        );

    }


    function test_NoConnectionSendLiquidity() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);

        uint256 amount = 100000;

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: MOCK_CHANNEL_ID,
            toVault: MOCK_REMOTE_VAULT,
            toAccount: MOCK_TO_ACCOUNT,
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });



        // Tested action: sendLiquidity without a valid connection
        vm.expectRevert(VaultNotConnected.selector);
        vault.sendLiquidity{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            amount,
            [uint256(0), uint256(0)],
            address(this),
            hex""
        );



        // Verify sendLiquidity works once the connection is set
        vault.setConnection(    // Set vault connection
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );
        CCI.connectNewChain(    // Connect the generalised incentives with a mock remote endpoint
            MOCK_CHANNEL_ID,
            convertEVMTo65(address(CCI)),
            abi.encode(address(GARP))
        );

        vault.sendLiquidity{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            amount,
            [uint256(0), uint256(0)],
            address(this),
            hex""
        );

    }


    function test_NoConnectionReceiveLiquidity() external {

        ICatalystV1Vault vault = ICatalystV1Vault(getTestConfig()[0]);



        // Tested action: receiveLiquidity without a valid connection
        vm.prank(address(CCI));
        vm.expectRevert(VaultNotConnected.selector);
        vault.receiveLiquidity(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            address(this),
            1000000,
            0,
            0,
            0,
            0
        );



        // Verify receiveLiquidity works once the connection is set
        vault.setConnection(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            true
        );

        vm.prank(address(CCI));
        vault.receiveLiquidity(
            MOCK_CHANNEL_ID,
            MOCK_REMOTE_VAULT,
            address(this),
            1000000,
            0,
            0,
            0,
            0
        );

    }
}
