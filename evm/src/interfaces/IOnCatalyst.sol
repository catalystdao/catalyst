//SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

interface ICatalystReceiver {
    /** 
     * @notice The callback from a catalyst call. To determine if the swap was an asset or liquidity swap, either the current balance should be checked or it should be encoded into data.
     * @dev If you want full finality (not just economical finality)
     * revert on underwritten == true.
     */
    function onCatalystCall(uint256 purchasedTokens, bytes calldata data, bool underwritten) external;
}
