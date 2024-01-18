// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solmate/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";
import { FixedPointMathLib } from "solmate/utils/FixedPointMathLib.sol";

abstract contract TestSecurityLimitLiquiditySwap is TestCommon, AVaultInterfaces {

    function test_security_limit_liquidity_swap() external {
        uint256 units = 4158883083359672064;
        address toAccount = makeAddr("toAccount"); 
        address[] memory vaults = getTestConfig();
        setUpChains(DESTINATION_IDENTIFIER);

        for (uint256 i = 0; i < vaults.length; ++i) {
            uint256 snapshot = vm.snapshot();
            address vault = vaults[i];
            // set connection between the vault and itself
            ICatalystV1Vault(vault).setConnection(
                DESTINATION_IDENTIFIER,
                convertEVMTo65(vault),
                true
            );

            uint256 target_token_index = 0;
            address target_token = ICatalystV1Vault(vault)._tokenIndexing(target_token_index);

            uint256 initial_target_token_balance = Token(target_token).balanceOf(vault);

            bytes memory fake_payload = constructSendLiquidity(vault, vault, toAccount, units);

            fake_payload = addGARPContext(keccak256(fake_payload), address(CCI), address(CCI), fake_payload);

            fake_payload = addMockContext(fake_payload);

            // Sign the fake payload
            (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), fake_payload);

            // Execute the fake payload
            GARP.processPacket(_metadata, toExecuteMessage, bytes32(abi.encode(address(this))));

            // Make a minout array that is 10. That should cover all test cases. The array is initialized with zeroes.
            uint256[] memory minOuts = new uint256[](10);
            // Withdraw the liquidity.
            uint256 pool_tokens = Token(vault).balanceOf(toAccount);
            vm.prank(toAccount);
            uint256[] memory withdrawal_amounts = ICatalystV1Vault(vault).withdrawAll(pool_tokens, minOuts);

            // For each token check that not more than 50% was transfered.
            for (uint256 j = 0; j < withdrawal_amounts.length; j++) {
                uint256 withdrawal_amount = withdrawal_amounts[j];
                address tkn = ICatalystV1Vault(vault)._tokenIndexing(j);
                if (tkn == address(0)) break;
                // Remember to add the withdrawn amount to the pool balance.
                uint256 pool_balance = Token(tkn).balanceOf(vault) + withdrawal_amount;
                assertGe(pool_balance / 2 * 10001 / 10000, withdrawal_amount, "More than expected withdrawn / liquidity swapped");
            }

            vm.revertTo(snapshot);
        }
    }
}

