// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import { TestCommon } from "../../TestCommon.t.sol";
import {Token} from "../../mocks/token.sol";

import { CatalystRouter } from "src/router/CatalystRouter.sol";
import { RouterParameters } from "src/router/base/RouterImmutables.sol";

contract TestRouterSendassetProfile is TestCommon {
    address internal constant ADDRESS_THIS = address(2);

    address TO_ACCOUNT = address(uint160(0xe00acc084f));

    address _REFUND_GAS_TO = TO_ACCOUNT;

    bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0xfee0eec191fa4f)));

    CatalystRouter router;

    function setUp() public virtual override {
        super.setUp();

        router = new CatalystRouter(
            RouterParameters({permit2: address(0), weth9: address(0x4200000000000000000000000000000000000006)})
        );
    }

    function pool1() internal returns(address vault1, address vault2) {
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
    
    function test_profile_sendasset() external {
        (address fromVault, address toVault) = pool1();
        address fromAsset = ICatalystV1Vault(fromVault)._tokenIndexing(0);
        address toAsset = ICatalystV1Vault(fromVault)._tokenIndexing(1);

        uint256 amount = uint256(0x11111111111111111);

        Token(fromAsset).approve(address(router), amount + 1);

        uint256 MINOUT = uint256(0x1111111111111111);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(TO_ACCOUNT),
            incentive: _INCENTIVE
        });

        bytes memory commands = abi.encodePacked(bytes1(0x1f), bytes1(0x01));

        bytes memory transfer_from = abi.encode(
            fromAsset,
            address(router),  // This is more expensive than using map but is better way to estimate costs.
            amount
        );

        bytes memory sendAsset = abi.encode(
            toVault,
            routeDescription,
            fromAsset,
            uint8(0),
            amount,
            MINOUT,
            ADDRESS_THIS,
            _getTotalIncentive(_INCENTIVE)
        );

        bytes[] memory inputs = new bytes[](2);
        inputs[0] = transfer_from;
        inputs[1] = sendAsset;

        vm.recordLogs();
        router.execute{value: _getTotalIncentive(_INCENTIVE)}(commands, inputs);

        Vm.Log[] memory entries = vm.getRecordedLogs();

        (bytes32 destinationIdentifier, bytes memory recipitent, bytes memory messageWithContext) = abi.decode(entries[4].data, (bytes32, bytes, bytes));

        bytes32 messageIdentifier = bytes32(this.sliceMemory(messageWithContext, 64+1, 64+1+32));

        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        vm.recordLogs();
        GARP.processMessage(_metadata, toExecuteMessage, FEE_RECIPITANT);

        entries = vm.getRecordedLogs();
        (destinationIdentifier, recipitent, messageWithContext) = abi.decode(entries[3].data, (bytes32, bytes, bytes));

        (_metadata, toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        GARP.processMessage(_metadata, toExecuteMessage, FEE_RECIPITANT);
    }
}