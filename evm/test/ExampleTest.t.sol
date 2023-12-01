// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import { Token } from "./mocks/token.sol";
import { TestCommon } from "./TestCommon.t.sol";
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";

contract ExampleTest is TestCommon {
    
  address vault;

  function setUp() public override {
    // Calls setup() on testCommon
    super.setUp();

    // Create relevant arrays for the vault.
    uint256 numTokens = 2;
    address[] memory assets = new address[](numTokens);
    uint256[] memory init_balances = new uint256[](numTokens);
    uint256[] memory weights = new uint256[](numTokens);

    // Deploy a token
    assets[0] = address(new Token("TEST", "TEST", 18, 1e6));
    init_balances[0] = 1000 * 1e18;
    weights[0] = 1;
    // Deploy another token
    assets[1] = address(new Token("TEST2", "TEST2", 18, 1e6));
    init_balances[1] = 1000 * 1e18;
    weights[1] = 1;

    // Set approvals.
    Token(assets[0]).approve(address(catFactory), init_balances[0]);
    Token(assets[1]).approve(address(catFactory), init_balances[1]);

    vault = catFactory.deployVault(
      address(volatileTemplate), assets, init_balances, weights, 10**18, 0, "Example Pool", "EXMP", address(CCI)
    );
  }

  function test_localswap() external {
    // Make an account for testing
    address alice = makeAddr("Alice");
    uint256 swapAmount = 100 * 10**18;

    // Get the token at index 0 from the vault
    address fromToken = ICatalystV1Vault(vault)._tokenIndexing(0);
    // Lets also get the to token while we are at it:
    address toToken = ICatalystV1Vault(vault)._tokenIndexing(1);

    Token(fromToken).transfer(alice, swapAmount);

    // Approve as alice.
    vm.prank(alice);
    Token(fromToken).approve(vault, swapAmount);

    uint256 minOut = 0;
    vm.prank(alice);
    uint256 swapReturn = ICatalystV1Vault(vault).localSwap(fromToken, toToken, swapAmount, minOut);
    
    assertEq(swapReturn, Token(toToken).balanceOf(alice), "Alice didn't get enough tokens");
  }
}