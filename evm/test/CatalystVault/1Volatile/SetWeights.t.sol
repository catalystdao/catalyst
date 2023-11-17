// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";

import "src/utils/FixedPointMathLib.sol";
import { Token } from "../../mocks/token.sol";
import { AVaultInterfaces } from "../AVaultInterfaces.t.sol";
import { CatalystVaultVolatile } from "src/CatalystVaultVolatile.sol";
import { TestInvariant } from "../Invariant.t.sol";


abstract contract TestSetWeights is Test, AVaultInterfaces, TestInvariant {
    using stdStorage for StdStorage;

    uint256 constant MIN_ADJUSTMENT_TIME = 7 days;
    uint256 constant MAX_ADJUSTMENT_TIME = 365 days;

    uint256 constant MAX_WEIGHTS_CHANGE = 10;

    uint256 changeFactorResolution = 100;



    // Helpers
    // ********************************************************************************************

    function mockWeights() internal returns(uint256[] memory) {

        uint256[] memory mockWeights = new uint256[](3);
        mockWeights[0] = 1501;
        mockWeights[1] = 3333;
        mockWeights[2] = 78654;

        return mockWeights;
    }

    function mockWeightChangeFactors() internal returns(uint256[] memory) {

        uint256[] memory changeFactors = new uint256[](3);

        changeFactors[0] = changeFactorResolution * 77 / 10;  // Increase the first weight        x7.7
        changeFactors[1] = changeFactorResolution / 5;        // Decrease the second weight       x0.2
        changeFactors[2] = changeFactorResolution * 1;        // Do not change the third weight   x1

        return changeFactors;
    }

    function mockNewWeights() internal returns(uint256[] memory) {

        uint256[] memory weights = mockWeights();
        uint256[] memory changeFactors = mockWeightChangeFactors();
        uint256[] memory newWeights = new uint256[](3);

        newWeights[0] = weights[0] * changeFactors[0] / changeFactorResolution;
        newWeights[1] = weights[1] * changeFactors[1] / changeFactorResolution;
        newWeights[2] = weights[2] * changeFactors[2] / changeFactorResolution;

        return newWeights;
    }

    function getCurrentWeights(address vault) internal returns (uint256[] memory) {

        address[] memory vaults = new address[](1);
        vaults[0] = vault;

        (uint256[] memory balances, uint256[] memory weights) = getBalances(vaults);

        return weights;
    }

    function forceSetWeights(
        CatalystVaultVolatile vault,
        uint256[] memory weights
    ) internal {

        for (uint256 i; i < weights.length; i++) {
            address token = vault._tokenIndexing(i);
            uint256 slot = stdstore
                .target(address(vault))
                .sig(vault._weight.selector)
                .with_key(token)
                .find();
            vm.store(address(vault), bytes32(slot), bytes32(weights[i]));
        }

        // Verify the weights have been correctly set
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        for (uint256 i; i < currentWeights.length; i++) {
            assertEq(currentWeights[i], weights[i]);
        }

    }

    function dummyLocalSwap(
        CatalystVaultVolatile vault
    ) internal {
        vault.localSwap(
            vault._tokenIndexing(0),
            vault._tokenIndexing(1),
            0,
            0
        );
    }



    // Tests
    // ********************************************************************************************

    function test_OnlyAdministrator(address alice) external {

        vm.assume(alice != address(this));

        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        uint256 currentTimestamp = block.timestamp;



        // Tested action
        vm.prank(alice);
        vm.expectRevert();
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);



        // Make sure the weights update works when called by the administrator
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);
    }


    function test_MinDuration() external {
        
        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        uint256 currentTimestamp = block.timestamp;



        // Tested action: update duration = min duration - 1
        vm.expectRevert();
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME - 1, newWeights);



        // Make sure the weights update works when the minimum duration is satisfied.
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME, newWeights);
    }


    function test_MaxDuration() external {
        
        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        uint256 currentTimestamp = block.timestamp;



        // Tested action: update duration = max duration + 1
        vm.expectRevert();
        vault.setWeights(currentTimestamp + MAX_ADJUSTMENT_TIME + 1, newWeights);



        // Make sure the weights update works when the maximum duration is satisfied.
        vault.setWeights(currentTimestamp + MAX_ADJUSTMENT_TIME, newWeights);
    }


    function test_MaxWeightsIncrease() external {
        
        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        // Set the last weight larger than the max allowed
        uint256 weightsCount = currentWeights.length;
        newWeights[weightsCount-1] = currentWeights[weightsCount-1] * MAX_WEIGHTS_CHANGE + 1;

        uint256 currentTimestamp = block.timestamp;



        // Tested action
        vm.expectRevert();
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);



        // Make sure the weights update works when the maximum change is satisfied
        newWeights[weightsCount-1] = currentWeights[weightsCount-1] * MAX_WEIGHTS_CHANGE;
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);
    }


    function test_MaxWeightsDecrease() external {
        
        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        // Set the last weight smaller than the min allowed
        uint256 weightsCount = currentWeights.length;
        newWeights[weightsCount-1] = currentWeights[weightsCount-1] / MAX_WEIGHTS_CHANGE - 1;

        uint256 currentTimestamp = block.timestamp;



        // Tested action
        vm.expectRevert();
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);



        // Make sure the weights update works when the maximum change is satisfied
        newWeights[weightsCount-1] = currentWeights[weightsCount-1] / MAX_WEIGHTS_CHANGE;
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);
    }


    function test_ZeroWeight() external {
        
        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        // Set the last weight to 0
        uint256 weightsCount = currentWeights.length;
        newWeights[weightsCount-1] = 0;

        uint256 currentTimestamp = block.timestamp;



        // Tested action
        vm.expectRevert();
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);



        // Make sure the weights update works when no 0-weights are specified
        newWeights[weightsCount-1] = currentWeights[weightsCount-1];
        vault.setWeights(currentTimestamp + MIN_ADJUSTMENT_TIME*2, newWeights);

    }


    function test_WeightsChangeCalculation() external {
        
        CatalystVaultVolatile vault = CatalystVaultVolatile(getTestConfig()[0]);

        forceSetWeights(vault, mockWeights());
        uint256[] memory currentWeights = getCurrentWeights(address(vault));
        uint256[] memory newWeights = mockNewWeights();

        uint256 startTimestamp = block.timestamp;
        uint256 weightsAdjustmentTime = MIN_ADJUSTMENT_TIME * 2;

        uint256 currentTimestamp = block.timestamp;




        // Tested action 1: Set the new weights
        vault.setWeights(currentTimestamp + weightsAdjustmentTime, newWeights);

        // Verify that the weights have not changed immediately
        uint256[] memory queriedWeights = getCurrentWeights(address(vault));
        for (uint256 i; i < queriedWeights.length; i++) {
            assertEq(queriedWeights[i], currentWeights[i]);
        }



        // Tested action 2: check weights at time = 30% change
        uint256 elapsedTime = weightsAdjustmentTime * 3 / 10;
        vm.warp(startTimestamp + elapsedTime);

        // Execute a local swap to force the weights to recalculate
        dummyLocalSwap(vault);

        // Verify that the weights have started updating
        queriedWeights = getCurrentWeights(address(vault));
        for (uint256 i; i < queriedWeights.length; i++) {
            int256 currentWeight = int256(currentWeights[i]);
            int256 targetWeight = int256(newWeights[i]);
            int256 weightDelta = targetWeight - currentWeight;

            uint256 expectedWeight = uint256(
                currentWeight + weightDelta * 3 / 10
            );
            assertEq(queriedWeights[i], expectedWeight);
        }


        
        // Tested action 3: check weights at time = 100% change
        vm.warp(startTimestamp + weightsAdjustmentTime);

        // Execute a local swap to force the weights to recalculate
        dummyLocalSwap(vault);

        // Verify that the weights have reached the desired values
        queriedWeights = getCurrentWeights(address(vault));
        for (uint256 i; i < queriedWeights.length; i++) {
            assertEq(queriedWeights[i], newWeights[i]);
        }


        
        // Tested action 4: check weights at time > 100% change
        vm.warp(startTimestamp + weightsAdjustmentTime * 12 / 10);

        // Execute a local swap to force the weights to recalculate
        dummyLocalSwap(vault);

        // Verify that the weights have not continued updating
        queriedWeights = getCurrentWeights(address(vault));
        for (uint256 i; i < queriedWeights.length; i++) {
            assertEq(queriedWeights[i], newWeights[i]);
        }

    }

}
