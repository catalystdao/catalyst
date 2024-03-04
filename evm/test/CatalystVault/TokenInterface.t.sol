// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";

abstract contract TestPoolTokenInterface is TestCommon, AVaultInterfaces {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    function alice_vault_token_deposit(address vault, address[] memory vault_tokens, address alice) internal returns(uint256 vaultTokens) {

        uint256 deposit_percentage = 15;
        uint256[] memory deposit_amounts = new uint256[](vault_tokens.length);

        for (uint256 i = 0; i < vault_tokens.length; ++i) {
            Token token = Token(vault_tokens[i]);
            deposit_amounts[i] = token.balanceOf(vault) * deposit_percentage / 100;

            token.transfer(alice, deposit_amounts[i]);
            vm.prank(alice);
            token.approve(vault, deposit_amounts[i]);
        }

        vm.prank(alice);
        vaultTokens = ICatalystV1Vault(vault).depositMixed(deposit_amounts, 0);
    }

    function get_vault_tokens(address vault) internal view returns(address[] memory vault_tokens) {
        uint256 numTokens;
        for (numTokens = 0; numTokens < 256; ++numTokens) {
            address token = ICatalystV1Vault(vault)._tokenIndexing(numTokens);
            if (token == address(0)) break;
        }
        vault_tokens = new address[](numTokens);
        for (uint256 i = 0; i < numTokens; ++i) {
            address token = ICatalystV1Vault(vault)._tokenIndexing(i);
            vault_tokens[i] = token;
        }
    }

    function test_vault_token_total_supply_query() external {
        address[] memory vaults = getTestConfig();
        address alice = makeAddr("alice");

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            address[] memory vault_tokens = get_vault_tokens(vault);
            uint256 mintedVaultTokens = alice_vault_token_deposit(vault, vault_tokens, alice);

            assertEq(
                Token(vault).balanceOf(alice),
                mintedVaultTokens,
                "Alice doesn't have expected tokens"
            );

            assertEq(
                Token(vault).totalSupply(),
                mintedVaultTokens + 10**18,
                "Total supply isn't expected"
            );  // NOTE: 10**18 is the vault token supply given to the vault deployer
        }
    }

    function test_vault_token_transfer() external {
        address[] memory vaults = getTestConfig();
        address alice = makeAddr("alice");
        address bob = makeAddr("bob");

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            address[] memory vault_tokens = get_vault_tokens(vault);
            uint256 mintedVaultTokens = alice_vault_token_deposit(vault, vault_tokens, alice);

            assertEq(
                Token(vault).balanceOf(alice),
                mintedVaultTokens
            );

            assertEq(
                Token(vault).balanceOf(bob),
                0
            );

            uint256 transfer_amount = 2 * mintedVaultTokens / 10;

            vm.prank(alice);
            vm.expectEmit();
            emit Transfer(alice, bob, transfer_amount);
            Token(vault).transfer(bob, transfer_amount);

            assertEq(
                Token(vault).balanceOf(alice),
                mintedVaultTokens - transfer_amount
            );

            assertEq(
                Token(vault).balanceOf(bob),
                0 + transfer_amount
            );
        }
    }

    function test_error_vault_token_transfer_no_balance() external {
        address[] memory vaults = getTestConfig();
        address alice = makeAddr("alice");
        address bob = makeAddr("bob");

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            address[] memory vault_tokens = get_vault_tokens(vault);
            uint256 mintedVaultTokens = alice_vault_token_deposit(vault, vault_tokens, alice);

            uint256 transfer_amount = 11 * mintedVaultTokens / 10;

            vm.prank(alice);
            vm.expectRevert();
            Token(vault).transfer(bob, transfer_amount);
        }
    }

    function test_vault_token_set_and_query_allowance(uint128 allowance) external {
        address[] memory vaults = getTestConfig();
        address alice = makeAddr("alice");
        address bob = makeAddr("bob");

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            address[] memory vault_tokens = get_vault_tokens(vault);
            alice_vault_token_deposit(vault, vault_tokens, alice);

            vm.prank(alice);
            vm.expectEmit();
            emit Approval(alice, bob, allowance);
            Token(vault).approve(bob, allowance);

            assertEq(
                Token(vault).allowance(alice, bob), allowance, "allowance not set"
            );
        }
    }

    function test_vault_token_transfer_from() external {
        address[] memory vaults = getTestConfig();
        address alice = makeAddr("alice");
        address bob = makeAddr("bob");
        address charlie = makeAddr("charlie");

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            address[] memory vault_tokens = get_vault_tokens(vault);
            uint256 mintedVaultTokens = alice_vault_token_deposit(vault, vault_tokens, alice);

            uint256 allowance_amount = 3 * mintedVaultTokens / 10;
            uint256 transfer_amount = 2 * mintedVaultTokens / 10;

            vm.prank(alice);
            Token(vault).approve(bob, allowance_amount);

            vm.prank(bob);
            vm.expectEmit();
            emit Transfer(alice, charlie, transfer_amount);
            Token(vault).transferFrom(alice, charlie, transfer_amount);

            assertEq(Token(vault).balanceOf(alice), mintedVaultTokens - transfer_amount, "balance after transfer from incorrect, fromee");
            assertEq(Token(vault).balanceOf(charlie), transfer_amount, "balance after transfer from incorrect, toee");
            assertEq(Token(vault).allowance(alice, bob), allowance_amount - transfer_amount, "allowance after transfer from incorrect");
        }
    }

    function test_error_vault_token_transfer_from_no_allowance() external {
        address[] memory vaults = getTestConfig();
        address alice = makeAddr("alice");
        address bob = makeAddr("bob");
        address charlie = makeAddr("charlie");

        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];

            address[] memory vault_tokens = get_vault_tokens(vault);
            uint256 mintedVaultTokens = alice_vault_token_deposit(vault, vault_tokens, alice);

            uint256 allowance_amount = 2 * mintedVaultTokens / 10;
            uint256 transfer_amount = 3 * mintedVaultTokens / 10;

            vm.prank(alice);
            Token(vault).approve(bob, allowance_amount);

            vm.prank(bob);
            vm.expectRevert();
            Token(vault).transferFrom(alice, charlie, transfer_amount);
        }
    }
}
