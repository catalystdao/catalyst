//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

error ExceedsSecurityLimit(uint256 overflow); // 0x12
error ReturnInsufficient(uint256 result, uint256 minimum); // 0x11 if on first argument or 0x21 if on second argument (only liquidity swap)
error VaultNotConnected(bytes32 connectionId, bytes toVault);
error WithdrawRatioNotZero();
error UnusedUnitsAfterWithdrawal(uint256 Units);
error EscrowAlreadyExists();
