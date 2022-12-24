// SPDX-License-Identifier: MIT

pragma solidity ^0.8.9;

import "./../IOnCatalyst.sol";

contract DummyTargetContract is ICatalystReceiver{ 

    event OnCatalystCallReceived(
        uint256 purchasedTokens,
        bytes data
    );

    function onCatalystCall(uint256 purchasedTokens, bytes calldata data) external {
        emit OnCatalystCallReceived(purchasedTokens, data);
    }
}