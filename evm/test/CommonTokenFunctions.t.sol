// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import {Token} from "./mocks/token.sol";

contract TestTokenFunctions is Test {

    string DEFAULT_POOL_SYMBOL;
    string DEFAULT_POOL_NAME;

    function getTokens(uint256 N) internal returns(address[] memory tokens) {
        tokens = new address[](N);
        for (uint256 i = 0; i < N; ++i) {
            tokens[i] = address(deployToken());
        }
    }

    function getTokens(uint256 N, uint256[] memory balances) internal returns(address[] memory tokens) {
        tokens = new address[](N);
        for (uint256 i = 0; i < N; ++i) {
            tokens[i] = address(deployToken(18, balances[i]));
        }
    }

    function approveTokens(address target, address[] memory tokens, uint256[] memory amounts) internal {
        for (uint256 i = 0; i < tokens.length; ++i) {
            Token(tokens[i]).approve(target, amounts[i]);
        }
    }

    function approveTokens(address target, address[] memory tokens) internal {
        uint256[] memory amounts = new uint256[](tokens.length);

        for (uint256 i = 0; i < amounts.length; ++i) {
            amounts[i] = 2**256 - 1;
        }

        approveTokens(target, tokens, amounts);
    }

    function verifyBalances(address target, address[] memory tokens, uint256[] memory amounts) internal {
        for (uint256 i = 0; i < tokens.length; ++i) {
            assertEq(
                Token(tokens[i]).balanceOf(target),
                amounts[i],
                "verifyBalances(...) failed"
            );
        }
    }

    function deployToken(
        string memory name,
        string memory symbol,
        uint8 decimals_,
        uint256 initialSupply
    ) internal returns (Token token) {
        return token = new Token(name, symbol, decimals_, initialSupply);
    }

    function deployToken(
        uint8 decimals_,
        uint256 initialSupply
    ) internal returns (Token token) {
        return token = deployToken("Token", "TKN", decimals_, initialSupply);
    }

    function deployToken() internal returns(Token token) {
        return deployToken(18, 1e6);
    }
    
}

