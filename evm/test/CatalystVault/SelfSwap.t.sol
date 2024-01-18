// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solmate/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

interface TF {
    function transfer(
        address to,
        uint256 amount
    ) external returns (bool);
}

abstract contract TestSelfSwap is TestCommon, AVaultInterfaces {
    uint256 private constant MARGIN_NUM = 10**18 + 10**10;
    uint256 private constant MARGIN_DENOM = 10**18;

    bytes32 private FEE_RECIPITANT = bytes32(uint256(uint160(0xfee0eec191fa4f)));

    function setupSendAsset(uint8 fromVaultIndex, uint8 fromAssetIndex, uint8 toVaultIndex, uint8 toAssetIndex, address[] memory vaults, uint32 swapSizePercentage, address toAccount) internal returns(uint256 units, uint256 amount, bytes memory messageWithContext) {

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];

        setConnection(fromVault, toVault, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);

        address fromToken = ICatalystV1Vault(fromVault)._tokenIndexing(fromAssetIndex);

        amount = getLargestSwap(fromVault, toVault, fromToken, ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex), true) * uint256(swapSizePercentage) / (2**32 - 1);

        Token(fromToken).approve(fromVault, amount);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE
        });

        vm.recordLogs();
        units = ICatalystV1Vault(fromVault).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            fromToken,
            toAssetIndex, 
            uint256(amount), 
            uint256(amount)/2, 
            toAccount,
            0,
            hex""
        );
        Vm.Log[] memory entries = vm.getRecordedLogs();

        (, , bytes memory messageWithContext_) = abi.decode(entries[1].data, (bytes32, bytes, bytes));

        return (units, amount, messageWithContext = messageWithContext_);
    }
    
    function test_self_swap(uint32 swapSizePercentage) external virtual {
        address[] memory vaults = getTestConfig();

        address toAccount = makeAddr("BOB");

        uint8 fromVaultIndex = 0;
        uint8 fromAssetIndex = 0;
        uint8 toVaultIndex = 0;
        uint8 toAssetIndex = 0;

        (uint256 units, uint256 fromAmount, bytes memory messageWithContext) = setupSendAsset(fromVaultIndex, fromAssetIndex, toVaultIndex, toAssetIndex, vaults, swapSizePercentage, toAccount);

        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        address toAsset = ICatalystV1Vault(vaults[toVaultIndex])._tokenIndexing(toAssetIndex);

        // Ensure that the call doesn't revert.
        vm.expectCall(
            toAsset,
            abi.encodeCall(
                TF.transfer,
                (
                    toAccount,
                    ICatalystV1Vault(vaults[toVaultIndex]).calcReceiveAsset(toAsset, units)
                )
            )
        );
        GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPITANT);

        
        uint256 toAmount = Token(toAsset).balanceOf(toAccount);

        if (fromAmount == 0 && toAmount == 0) return;
        // Allow a very small error.
        assertGt(fromAmount * MARGIN_NUM / MARGIN_DENOM, toAmount, "More out than in");
    }
}

