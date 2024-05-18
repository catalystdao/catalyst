//SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

error EscrowAlreadyExists(); // 0xed778779
error ExceedsSecurityLimit(); // 0x7c1e66d0
error NotEnoughGas(); // 0xdd629f86
error ReturnInsufficient(uint256,uint256); // 0x24557f05
error UnusedUnitsAfterWithdrawal(uint256); // 0x0289311f
error VaultNotConnected(); // 0x2c64c1b2
error WithdrawRatioNotZero(); // 0xb8003bfa