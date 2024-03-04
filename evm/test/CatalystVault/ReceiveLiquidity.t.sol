// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/CatalystVaultVolatile.sol";
import "solady/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

interface TF {
    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) external returns (bool);

    function transfer(
        address to,
        uint256 amount
    ) external returns (bool);
}

abstract contract TestReceiveLiquidity is TestCommon, AVaultInterfaces {
    uint256 private constant MARGIN_NUM = 1;
    uint256 private constant MARGIN_DENOM = 1e12;

    bytes32 private FEE_RECIPITANT = bytes32(uint256(uint160(0xfee0eec191fa4f)));

    function setupSendLiquidity(uint8 fromVaultIndex, uint8 toVaultIndex, address[] memory vaults, uint32 swapSizePercentage, address toAccount) internal returns(uint256 units, uint256 amount, bytes memory messageWithContext) {

        address fromVault = vaults[fromVaultIndex];
        address toVault = vaults[toVaultIndex];

        setConnection(fromVault, toVault, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);

        amount = Token(fromVault).balanceOf(address(this)) * swapSizePercentage / (2**32 - 1);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(toVault),
            toAccount: convertEVMTo65(toAccount),
            incentive: _INCENTIVE,
            deadline: uint64(0)
        });

        vm.recordLogs();
        units = ICatalystV1Vault(fromVault).sendLiquidity{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            amount,
            [uint256(0), uint256(0)],
            toAccount,
            hex""
        );
        Vm.Log[] memory entries = vm.getRecordedLogs();

        (, , bytes memory messageWithContext_) = abi.decode(entries[2].data, (bytes32, bytes, bytes));

        return (units, amount, messageWithContext = messageWithContext_);
    }
    
    function test_receiveLiquidity(uint32 swapSizePercentage, address toAccount) external virtual {
        vm.assume(toAccount != address(0));
        vm.assume(swapSizePercentage != type(uint32).max);
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex = 1;

        (uint256 units, uint256 fromAmount, bytes memory messageWithContext) = setupSendLiquidity(fromVaultIndex, toVaultIndex, vaults, swapSizePercentage, toAccount);

        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);

        // Ensure that the call doesn't revert.
        vm.expectCall(
            vaults[toVaultIndex],
            abi.encodeCall(
                CatalystVaultVolatile.receiveLiquidity,
                (
                    DESTINATION_IDENTIFIER,
                    convertEVMTo65(vaults[fromVaultIndex]),
                    toAccount,
                    units,
                    uint256(0),
                    uint256(0),
                    fromAmount,
                    uint32(1)
                )
            )
        );
        GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPITANT);
    }
}

