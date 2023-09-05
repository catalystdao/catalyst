// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

contract TestDefaultGovernanceFee is TestCommon {
    
    function test_set_default_governance_fee_storage(uint64 fee) external {
        vm.assume(fee <= 75e16);
        // Check that it is initially 0.
        assertEq(catFactory._defaultGovernanceFee(), 0);
        
        vm.prank(catFactory.owner());
        catFactory.setDefaultGovernanceFee(fee);

        // Check that the storage updated.
        assertEq(catFactory._defaultGovernanceFee(), fee);
    }

    function test_error_large_governance_fee() external {
        assertEq(catFactory._defaultGovernanceFee(), 0);

        uint64 fee = 75e16 + 1;
        
        vm.prank(catFactory.owner());
        vm.expectRevert();
        catFactory.setDefaultGovernanceFee(fee);

        // Check that the storage did not updated.
        assertEq(catFactory._defaultGovernanceFee(), 0);
    }

    function test_set_default_governance_fee_storage_only_owner(address caller, uint256 fee) external {
        vm.assume(caller != catFactory.owner());

        vm.prank(caller);
        vm.expectRevert("Ownable: caller is not the owner");
        catFactory.setDefaultGovernanceFee(fee);

        // Check that the storage isn't updated.
        assertEq(catFactory._defaultGovernanceFee(), 0);
    }
}

