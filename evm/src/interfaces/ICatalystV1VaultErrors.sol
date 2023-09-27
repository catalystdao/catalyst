//SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.17;

error ExceedsSecurityLimit();  // 7c1e66d
error ReturnInsufficientOnReceive();  // 52dae07
error ReturnInsufficient(uint256 result, uint256 minimum);  // 24557f0
error VaultNotConnected();  // 2c64c1b
error WithdrawRatioNotZero();  // b8003bf
error UnusedUnitsAfterWithdrawal(uint256 Units);  // 0289311
error EscrowAlreadyExists();  // ed77877