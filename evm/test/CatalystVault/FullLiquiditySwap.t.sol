// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import { ICatalystV1Vault } from "src/ICatalystV1Vault.sol";
import { ICatalystV1VaultEvents } from "src/interfaces/ICatalystV1VaultEvents.sol";
import { CatalystVaultAmplified } from "src/CatalystVaultAmplified.sol";
import { VaultNotConnected } from "src/interfaces/ICatalystV1VaultErrors.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import { FixedPointMathLib as Math } from "src/utils/FixedPointMathLib.sol";

import "forge-std/Test.sol";
import { TestCommon } from "test/TestCommon.t.sol";
import { Token } from "test/mocks/token.sol";
import { AVaultInterfaces } from "test/CatalystVault/AVaultInterfaces.t.sol";
import { TestInvariant } from "test/CatalystVault/Invariant.t.sol";


function queryAssetCount(ICatalystV1Vault vault) returns (uint256) {
    uint256 tokenCount = 0;
    for (uint256 i; true; i++) {
        address token = vault._tokenIndexing(i);
        if (token == address(0)) return tokenCount;
        else tokenCount += 1;
    }
}

function queryVaultWeightsSum(ICatalystV1Vault vault) returns (uint256) {
    uint256 weightsSum = 0;
    for (uint256 i; true; i++) {
        uint256 weight = vault._weight(vault._tokenIndexing(i));
        if (weight == 0) return weightsSum;
        else weightsSum += weight;
    }
}

abstract contract TestFullLiquiditySwap is TestCommon, AVaultInterfaces {


    // Helpers
    // ********************************************************************************************

    function computeExpectedLiquiditySwap(
        bool amplified,
        ICatalystV1Vault fromVault,
        ICatalystV1Vault toVault,
        uint256 swapAmount
    ) private returns (uint256, uint256) {

        uint256 fromVaultTotalSupply = Token(address(fromVault)).totalSupply();
        uint256 toVaultTotalSupply = Token(address(toVault)).totalSupply();

        uint256 units;
        uint256 toAmount;
        if (!amplified) {

            uint256 fromVaultWeightsSum = queryVaultWeightsSum(fromVault);
            uint256 toVaultWeightsSum = queryVaultWeightsSum(toVault);

            units = uint256(Math.lnWad(int256(
                fromVaultTotalSupply * Math.WAD / (fromVaultTotalSupply - swapAmount)
            ))) * fromVaultWeightsSum;

            uint256 shareWad = Math.WAD - uint256(Math.expWad(
                -int256(units/toVaultWeightsSum)
            ));

            toAmount = uint256(
                toVaultTotalSupply * shareWad / (Math.WAD - shareWad)
            );
        }
        else {

            CatalystVaultAmplified ampFromVault = CatalystVaultAmplified(address(fromVault));
            CatalystVaultAmplified ampToVault = CatalystVaultAmplified(address(toVault));

            int256 oneMinusAmp = ampFromVault._oneMinusAmp();
            assertEq(oneMinusAmp, ampToVault._oneMinusAmp());   // Sanity check

            int256 fromVaultAssetCount = int256(queryAssetCount(fromVault));
            int256 toVaultAssetCount = int256(queryAssetCount(toVault));

            // NOTE: fromVaultB0 and toVaultB0 are in WAD terms
            int256 fromVaultB0 = int256(ampFromVault.computeBalance0());
            int256 toVaultB0 = int256(ampToVault.computeBalance0());

            units = uint256(fromVaultAssetCount * (
                Math.powWad(
                    fromVaultB0 * (int256(swapAmount + fromVaultTotalSupply)) / int256(fromVaultTotalSupply),
                    oneMinusAmp
                )
                - Math.powWad(fromVaultB0, oneMinusAmp)
            ));

            int256 wpt = Math.powWad(
                Math.powWad(toVaultB0, oneMinusAmp) + int256(units)/toVaultAssetCount,
                Math.WADWAD / oneMinusAmp
            ) - toVaultB0;
            if (wpt < 0) wpt = 0;   // 'wpt' may go negative if 'units' is very small or 0 (calculation error)

            toAmount = uint256(wpt) * toVaultTotalSupply / uint256(toVaultB0) ;   // NOTE: divide first to prevent a calculation overflow

        }

        return (units, toAmount);
    }

    function findLog(Vm.Log[] memory logs, bytes32 logSelector) private returns(Vm.Log memory) {
        
        Vm.Log memory log;

        for (uint256 i; i < logs.length; i++) {
            if (logs[i].topics[0] == logSelector) return logs[i];
        }

        revert("Log not found.");
    }

    bytes32 private constant FEE_RECIPIENT = bytes32(uint256(uint160(0xfee0eec191fa4f)));



    // Tests
    // ********************************************************************************************

    function test_FullLiquiditySwap(uint32 depositPercentage, uint32 swapPercentage) external {

        vm.assume(depositPercentage < 20000);
        vm.assume(swapPercentage < 10000);
        uint256 percentageBase = 10000;


        // Test config
        address[] memory vaults = getTestConfig();
        require(vaults.length >= 2, "Not enough vaults defined");

        uint8 fromVaultIndex = 0;
        uint8 toVaultIndex   = 1;

        address swapper = address(1);
        vm.deal(swapper, 1 ether);     // Fund account for incentive payment

        ICatalystV1Vault fromVault = ICatalystV1Vault(vaults[fromVaultIndex]);
        ICatalystV1Vault toVault   = ICatalystV1Vault(vaults[toVaultIndex]);

        setConnection(address(fromVault), address(toVault), DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);


        // Deposit to get vault tokens
        uint256 fromVaultAssetCount = queryAssetCount(fromVault);
        uint256[] memory depositAmounts = new uint256[](fromVaultAssetCount);

        for (uint256 i; i < fromVaultAssetCount; i++) {

            Token token = Token(fromVault._tokenIndexing(i));
            uint256 depositAmount = token.balanceOf(address(fromVault)) * depositPercentage / percentageBase;
            depositAmounts[i] = depositAmount;

            token.transfer(swapper, depositAmount);

            vm.prank(swapper);
            token.approve(address(fromVault), depositAmount);
        }

        uint256 expectedVaultTokens = Token(address(fromVault)).totalSupply()
            * depositPercentage / percentageBase;

        vm.prank(swapper);
        uint256 fromVaultTokens = fromVault.depositMixed(
            depositAmounts,
            expectedVaultTokens * 999 / 1000    // Minimum output
        );


        // Perform the liquidity swap
        uint256 swappedVaultTokens = fromVaultTokens * swapPercentage / percentageBase;

        (uint256 expectedUnits, uint256 expectedReturn) = computeExpectedLiquiditySwap(
            amplified,
            fromVault,
            toVault,
            swappedVaultTokens
        );

        vm.recordLogs();
        vm.prank(swapper);

        ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
            chainIdentifier: DESTINATION_IDENTIFIER,
            toVault: convertEVMTo65(address(toVault)),
            toAccount: convertEVMTo65(swapper),
            incentive: _INCENTIVE
        });

        uint256 outputUnits = ICatalystV1Vault(fromVault).sendLiquidity{value: _getTotalIncentive(_INCENTIVE)}(
            routeDescription,
            swappedVaultTokens,
            [expectedReturn * 99 / 100, uint256(0)],
            swapper,
            hex""
        );

        // Verify the 'sent' vault tokens have been burnt
        assertEq(
            Token(address(fromVault)).balanceOf(swapper),
            fromVaultTokens - swappedVaultTokens
        );


        // Check whether the swap passes the security limit.
        uint256 toVaultCapacity = toVault.getUnitCapacity();
        bool securityLimitPasses = false;
        if (!amplified) {
            securityLimitPasses = toVaultCapacity >= outputUnits;
        }
        else {
            int256 oneMinusAmp = CatalystVaultAmplified(address(toVault))._oneMinusAmp();

            int256 toVaultBalance0 = int256(CatalystVaultAmplified(address(toVault)).computeBalance0());
            int256 NtoVaultBalance0 = int256(queryAssetCount(toVault)) * toVaultBalance0;

            int256 NtoVaultBalance0Ampped = Math.powWad(NtoVaultBalance0, oneMinusAmp);

            if (NtoVaultBalance0Ampped >= int256(outputUnits)) {

                // NOTE: 'powWad' fails for x=0
                int256 diffAmped = (NtoVaultBalance0Ampped - int256(outputUnits)) * int256(Math.WAD) / NtoVaultBalance0Ampped;
                uint256 diff;
                if (diffAmped == 0) diff = 0;
                else diff = uint256(Math.powWad(diffAmped, Math.WADWAD / oneMinusAmp));

                uint256 expectedEffectiveSwapYield = uint256(
                    Math.mulWadDown(
                        uint256(NtoVaultBalance0),
                        Math.WAD - diff
                    ) / Math.WAD
                );

                securityLimitPasses = toVaultCapacity >= expectedEffectiveSwapYield * 11 / 10;  // Allow some extra margin for calculation errors
            }
            // Otherwise securityLimitPasses = false
        }

        // Complete the execution on the destination chain
        Vm.Log[] memory entries = vm.getRecordedLogs();
        (, , bytes memory crossChainPacket) = abi.decode(entries[2].data, (bytes32, bytes, bytes));

        vm.recordLogs();
        (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), crossChainPacket);
        GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPIENT);


        // Verify the swap executed on the destination
        entries = vm.getRecordedLogs();

        if (!securityLimitPasses) {

            // If the security limit disallows the swap, verify the swap execution reverts.

            (, , crossChainPacket) = abi.decode(entries[entries.length-1].data, (bytes32, bytes, bytes));

            // The mock messaging implementation has 2 initial addresses (32 bytes) before the
            // GeneralisedIncentives payload. Within the GI payload, the Catalyst payload starts on byte 144.
            bytes1 resultId = crossChainPacket[32 + 32 + 144];

            if (amplified && resultId == 0) {
                // If the swap did not fail for amplified vaults, verify that the security limit is
                // almost completely exhausted.
                assert(toVault.getUnitCapacity() * 1000 / toVaultCapacity <= 15);
            }
            else {
                assertEq(resultId, bytes1(0x11));   // 0x11 id corresponds to the 'ExceedsSecurityLimit' error (see CatalystChainInterface.sol)
            }

            // Finish test case
            return;
        }

        // Verify the 'received' vault tokens have been minted
        Vm.Log memory receiveLiquidityLog = findLog(entries, ICatalystV1VaultEvents.ReceiveLiquidity.selector);

        (, , , , uint256 purchasedVaultTokens, , ) = abi.decode(
            receiveLiquidityLog.data,
            (bytes32, bytes, address, uint256, uint256, uint256, uint256)
        );

        assertEq(
            Token(address(toVault)).balanceOf(swapper),
            purchasedVaultTokens
        );

        // Verify the liquidity swap calculation
        require(
            purchasedVaultTokens <= expectedReturn * 1000001 / 1000000,
            "Liquidity swap returns more than expected."
        );
        require(
            purchasedVaultTokens >= expectedReturn * 999 / 1000,
            "Liquidity swap returns less than expected."
        );

    }
}
