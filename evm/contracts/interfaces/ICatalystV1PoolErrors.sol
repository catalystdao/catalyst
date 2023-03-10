//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

error ExceedsSecurityLimit(uint256 overflow);
error ReturnInsufficient(uint256 result, uint256 minimum);
error PoolNotConnected(bytes32 connectionId, bytes32 toPool);
error WithdrawRatioNotZero();