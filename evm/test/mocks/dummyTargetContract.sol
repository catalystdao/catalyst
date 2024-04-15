// SPDX-License-Identifier: MIT

pragma solidity ^0.8.19;

import "./../../src/interfaces/IOnCatalyst.sol";

contract DummyTargetContract is ICatalystReceiver {
    event OnCatalystCallReceived(uint256 purchasedTokens, bytes data, bool underwritten);

    function onCatalystCall(uint256 purchasedTokens, bytes calldata data, bool underwritten)
        external
    {
        emit OnCatalystCallReceived(purchasedTokens, data, underwritten);
    }
}
