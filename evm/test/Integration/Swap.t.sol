// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";
import {Token} from "../mocks/token.sol";
import "../../src/ICatalystV1Vault.sol";
import { ICatalystV1Structs } from "../../src/interfaces/ICatalystV1VaultState.sol";


interface RA {
    function receiveAsset(
        bytes32 channelId,
        bytes calldata fromVault,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod
    ) external;
}

contract TestSwapIntegration is TestCommon {

    event Message(
        bytes32 destinationIdentifier,
        bytes recipitent,
        bytes message
    );

    event SendAsset(
        bytes32 channelId,
        bytes toVault,
        bytes toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 fromAmount,
        uint256 minOut,
        uint256 units,
        uint256 fee,
        uint16 underwriteIncentiveX16
    );

    event SendAssetSuccess(
        bytes32 channelId,
        bytes toAccount,
        uint256 Units,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ); 

    bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0xfee0eec191fa4f)));

    address TO_ACCOUNT = address(uint160(0xe00acc084f));

    address _REFUND_GAS_TO = TO_ACCOUNT;

    function pool1() internal returns(address vault1, address vault2) {

        // Deploy tokens.
        address[] memory tokens1 = getTokens(3);
        address[] memory tokens2 = getTokens(1);
        approveTokens(address(catFactory), tokens1);
        approveTokens(address(catFactory), tokens2);

        // Deploy a volatile vault
        uint256[] memory amounts1 = new uint256[](3);
        amounts1[0] = 100*10**18; amounts1[1] = 200*10**18; amounts1[2] = 300*10**18;
        uint256[] memory weights1 = new uint256[](3);
        weights1[0] = 1; weights1[1] = 1; weights1[2] = 1;
        vault1 = deployVault(
            tokens1,
            amounts1,
            weights1,
            10**18,
            0
        );

        // Deploy a volatile vault
        uint256[] memory amounts2 = new uint256[](1);
        amounts2[0] = 100*10**18;
        uint256[] memory weights2 = new uint256[](1);
        weights2[0] = 1;
        vault2 = deployVault(
            tokens2,
            amounts2,
            weights2,
            10**18,
            0
        );
        
        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);
    }

    function pool2() internal returns(address vault1, address vault2) {
        // Deploy tokens. 
        address[] memory tokens = getTokens(3);
        approveTokens(address(catFactory), tokens);

        // Deploy an amplified vault
        uint256[] memory amounts = new uint256[](3);
        amounts[0] = 100*10**18; amounts[1] = 200*10**18; amounts[2] = 300*10**18;
        uint256[] memory weights = new uint256[](3);
        weights[0] = 1; weights[1] = 1; weights[2] = 1;
        amounts[0] = 100*10**18; amounts[1] = 200*10**18; amounts[2] = 300*10**18;
        weights[0] = 1; weights[1] = 1; weights[2] = 1;
        vault1 = deployVault(
            tokens,
            amounts,
            weights,
            10**18 / 2,
            0
        );

        vault2 = vault1;

        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);
    }

    function test_cross_chain_swap_volatile() external {
        uint256 amount = 10**18;
        (address vault1, address vault2) = pool1();
        t_cross_chain_swap(vault1, vault2, amount);
    }

    function test_cross_chain_swap_amplified() external {
        uint256 amount = 10**18;
        (address vault1, address vault2) = pool2();
        t_cross_chain_swap(vault1, vault2, amount);
    }
    
    function t_cross_chain_swap(address fromVault, address toVault, uint256 amount) internal {
        address tkn = ICatalystV1Vault(fromVault)._tokenIndexing(0);

        Token(tkn).approve(fromVault, amount);

        // TODO: this message, we need to add the units.
        // vm.expectEmit();
        // emit Message(
        //     CHANNEL_ID,
        //     hex"0000000000000000000000005991a2df15a8f6a256d3ec51e99254cd3fb576a9",
        //     hex"80000000000000000000000000000000000000000000000000000000001231230000000000000000000000000000000000000000000000000000000000000539003fd017ce8d9e2f46a0d62e4cb993736c47339aad1b29a35c05e653dd3964d4e9140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c7183455a4c133ae270771860664b6b7ec320bb1140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c7183455a4c133ae270771860664b6b7ec320bb1000000124c5f00140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000104fbc016f4bb334d775a19e8a6510109ac63e00140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000037eda3adb1198021a9b2e88c22b464fd38db3f3140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001e0f3000000000000000000000000000000000000000000000000400c33a879f0c1ba000000000000000000000000000000000000000000000000000000000000000c8d00000000000000000000000000000000000000000000021e19e0c9bab2400000140000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a0cb889707d426a7a386870a03bc70d1b0697598000000010000"
        // );

        uint256 MINOUT = 3213;

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(TO_ACCOUNT),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });
        
        uint256 snapshotId = vm.snapshot();
        uint256 UNITS = ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            tkn,
            0,
            amount,
            MINOUT,
            TO_ACCOUNT,
            0,
            hex""
        );
        vm.revertTo(snapshotId);


        vm.expectEmit();
        emit SendAsset(
            DESTINATION_IDENTIFIER,
            convertEVMTo65(toVault),
            convertEVMTo65(TO_ACCOUNT),
            tkn,
            0,
            amount,
            MINOUT,
            UNITS,
            0,
            0
        );

        vm.expectCall(
            address(CCI),
            _getTotalIncentive(_INCENTIVE), // value
            abi.encodeCall(
                CCI.sendCrossChainAsset,
                (
                    routeDescription,
                    0,
                    UNITS,
                    MINOUT,
                    amount,
                    tkn,
                    0,
                    hex""
                )
            )
        );

        vm.recordLogs();
        ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            tkn,
            0,
            amount,
            MINOUT,
            TO_ACCOUNT,
            0,
            hex""
        );  

        // The message is event 2 while bounty place is number 1. (index 1, 0)
        Vm.Log[] memory entries = vm.getRecordedLogs();



        (bytes32 destinationIdentifier, bytes memory recipitent, bytes memory messageWithContext) = abi.decode(entries[1].data, (bytes32, bytes, bytes));

        bytes32 messageIdentifier = bytes32(this.sliceMemory(messageWithContext, 64+1, 64+1+32));

        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        vm.expectCall(
            address(CCI),
            abi.encodeCall(
                CCI.receiveMessage,
                (
                    DESTINATION_IDENTIFIER,
                    messageIdentifier,
                    convertEVMTo65(address(CCI)),
                    this.sliceMemory(messageWithContext, 64+177)
                )
            )
        );

        vm.expectCall(
            address(toVault),
            abi.encodeCall(
                RA.receiveAsset,
                (
                    DESTINATION_IDENTIFIER,
                    convertEVMTo65(fromVault),
                    0,
                    TO_ACCOUNT,
                    UNITS,
                    MINOUT,
                    amount,
                    convertEVMTo65(tkn),
                    1
                )
            )
        );

        vm.recordLogs();
        GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPITANT);

        entries = vm.getRecordedLogs();
        (destinationIdentifier, recipitent, messageWithContext) = abi.decode(entries[3].data, (bytes32, bytes, bytes));

        (_metadata, toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        vm.expectEmit();
        emit SendAssetSuccess(
            DESTINATION_IDENTIFIER,
            convertEVMTo65(TO_ACCOUNT),
            UNITS,
            amount,
            tkn,
            1
        );

        GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPITANT);
    }
}