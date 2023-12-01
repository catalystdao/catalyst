// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "./HandleError.m.sol";

contract TestHandleError is Test {

    ExposeHandleError exposeHandleError;
    
    function setUp() external {
        exposeHandleError = new ExposeHandleError();
    }

    function test_handle_error_0x11() external {
        bytes memory err = abi.encodeWithSelector(ExceedsSecurityLimit.selector);
        bytes1 code = exposeHandleError.handleError(err);

        assertEq(
            code,
            bytes1(0x11)
        );
    }
    function test_handle_error_0x12() external {
        bytes memory err = abi.encodeWithSelector(ReturnInsufficient.selector);
        bytes1 code = exposeHandleError.handleError(err);

        assertEq(
            code,
            bytes1(0x12)
        );
    }

    function test_handle_error_0x13() external {
        bytes memory err = abi.encodeWithSelector(VaultNotConnected.selector);
        bytes1 code = exposeHandleError.handleError(err);

        assertEq(
            code,
            bytes1(0x13)
        );
    }

    function test_handle_error_0x10(bytes memory err) external {
        bytes8 hashOfError = bytes8(err);
        vm.assume(
            hashOfError != bytes8(abi.encodeWithSelector(ExceedsSecurityLimit.selector))
        );
        vm.assume(
            hashOfError != bytes8(abi.encodeWithSelector(ReturnInsufficient.selector))
        );
        vm.assume(
            hashOfError != bytes8(abi.encodeWithSelector(VaultNotConnected.selector))
        );
        bytes1 code = exposeHandleError.handleError(err);

        assertEq(
            code,
            bytes1(0x10)
        );
    }

}

