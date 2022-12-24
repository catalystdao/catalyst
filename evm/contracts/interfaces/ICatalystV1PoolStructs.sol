//SPDX-License-Identifier: Unlicsened
pragma solidity ^0.8.17;

struct TokenEscrow {
    uint256 amount;
    address token;
}

struct LiquidityEscrow {
    uint256 poolTokens;
}