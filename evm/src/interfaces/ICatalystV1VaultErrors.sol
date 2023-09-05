//SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.16;

error ExceedsSecurityLimit();
error ReturnInsufficientOnReceive();
error ReturnInsufficient(uint256 result, uint256 minimum);
error VaultNotConnected(bytes32 connectionId, bytes toVault);
error WithdrawRatioNotZero();
error UnusedUnitsAfterWithdrawal(uint256 Units);
error EscrowAlreadyExists();
