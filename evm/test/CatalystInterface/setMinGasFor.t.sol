// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";

contract TestSetMinGasFor is TestCommon {

    bytes32 defaultExampleChainIdentifier = bytes32(uint256(1231));

    uint48 defaultExampleMinGas = 444;
    
    function test_min_gas_for_storage(bytes32 chainIdentifier, uint48 minGas) external {
        // Check that it is initially 0.
        assertEq(CCI.minGasFor(chainIdentifier), 0);
        
        vm.prank(CCI.owner());
        CCI.setMinGasFor(chainIdentifier, minGas);

        // Check that the storage updated.
        assertEq(CCI.minGasFor(chainIdentifier), minGas);
    }

    function test_min_gas_for_storage_only_owner(address caller) external {
        vm.assume(caller != CCI.owner());

        vm.prank(caller);
        vm.expectRevert("Ownable: caller is not the owner");
        CCI.setMinGasFor(defaultExampleChainIdentifier, defaultExampleMinGas);

        // Check that the storage isn't updated.
        assertNotEq(CCI.minGasFor(defaultExampleChainIdentifier), defaultExampleMinGas);
    }
}

