// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/CatalystChainInterface.sol";

contract ExposeHandleError {
    constructor() {}

    function handleError(bytes memory err) pure external returns(bytes1) {
        return _handleError(err);
    }

    function _handleError(bytes memory err) pure internal returns (bytes1) {
        // To safe on gas, only examine the first 32 bytes.
        bytes32 errorIdentifier = bytes32(err);
        // We can use memory sclies to get better insight into exactly the error which occured.
        // This would also allow us to reuse events.
        // However, it looks like it will significantly increase gas costs so this works for now.
        // It looks like Solidity will improve their error catch implementation which will replace this.
        if (bytes32(abi.encodeWithSelector(ExceedsSecurityLimit.selector)) == errorIdentifier) return 0x11;
        if (bytes32(abi.encodeWithSelector(ReturnInsufficientOnReceive.selector)) == errorIdentifier) return 0x12;
        if (bytes32(abi.encodeWithSelector(VaultNotConnected.selector)) == errorIdentifier) return 0x13;
        return 0x10; // unknown error.
    }
}

