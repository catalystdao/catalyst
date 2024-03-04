// SPDX-License-Identifier: MIT

pragma solidity ^0.8.19;

import {ERC20} from "solady/tokens/ERC20.sol";

contract TokenLens {
  struct AccountBalance {
    uint256 balance;
    TokenBalance[] tokenBalances;
  }

  struct TokenBalance {
    address token;
    uint256 amount;
  }

  struct TokenInformation {
    address token;
    string name;
    string symbol;
    uint256 decimals;
  }

  function fetchAccountBalances(
    address owner,
    address[] calldata tokens
  ) external view returns (AccountBalance memory balance) {
    balance = AccountBalance({balance: address(msg.sender).balance, tokenBalances: fetchTokenBalances(owner, tokens)});
  }

  function fetchTokenBalances(
    address owner,
    address[] calldata tokens
  ) public view returns (TokenBalance[] memory balances) {
    balances = new TokenBalance[](tokens.length);
    for (uint256 i; i < tokens.length; ) {
      address token = tokens[i];
      balances[i] = TokenBalance({token: token, amount: ERC20(token).balanceOf(owner)});
      unchecked {
        ++i;
      }
    }
  }

  function fetchTokenAllowances(
    address owner,
    address spender,
    address[] calldata tokens
  ) external view returns (TokenBalance[] memory allowances) {
    allowances = new TokenBalance[](tokens.length);
    for (uint256 i; i < tokens.length; ) {
      address token = tokens[i];
      allowances[i] = TokenBalance({token: token, amount: ERC20(token).allowance(owner, spender)});
      unchecked {
        ++i;
      }
    }
  }

  function fetchTokenInformations(
    address[] calldata tokens
  ) external view returns (TokenInformation[] memory tokenInformation) {
    tokenInformation = new TokenInformation[](tokens.length);
    for (uint256 i; i < tokens.length; ) {
      tokenInformation[i] = fetchTokenInformation(tokens[i]);
      unchecked {
        ++i;
      }
    }
  }

  function fetchTokenInformation(address token) public view returns (TokenInformation memory tokenInformation) {
    ERC20 tokenContract = ERC20(token);
    tokenInformation = TokenInformation({
      token: token,
      name: tokenContract.name(),
      symbol: tokenContract.symbol(),
      decimals: tokenContract.decimals()
    });
  }
}
