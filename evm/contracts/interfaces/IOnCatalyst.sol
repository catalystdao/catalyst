//SPDX-License-Identifier: Unlicsened

pragma solidity >=0.8.17 <0.9.0;

interface ICatalystReceiver {
    /**
     * @notice Deposits a symmetrical number of tokens such that baseAmount of pool tokens are minted.
     * This doesn't change the pool price.
     * @dev Requires approvals for all tokens within the pool.
     * @param data The number of pool tokens to mint.
     */
    function onCatalystCall(uint256 purchasedTokens, bytes calldata data)
        external;
}
