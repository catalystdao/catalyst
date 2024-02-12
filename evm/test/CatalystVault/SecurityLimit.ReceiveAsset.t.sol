// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";
import { ICatalystV1Structs } from "src/interfaces/ICatalystV1VaultState.sol";
import {Token} from "../mocks/token.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";
import { FixedPointMathLib } from "solady/utils/FixedPointMathLib.sol";

abstract contract TestSecurityLimitAssetSwap is TestCommon, AVaultInterfaces {
    // This is a test to see if the security limit behaves as intended:
    // That is, no more than 50% of the vault should be extractable.
    // Because of how package handling works, we are fine if the swap reverts and there is not execution.
    function test_security_limit_send_asset(uint256 units) external {
        units = 4158883083359672064;
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

            bytes memory fake_payload = constructSendAsset(vault, vault, toAccount, units, 0);

            fake_payload = addGARPContext(keccak256(fake_payload), address(CCI), address(CCI), fake_payload);

            fake_payload = addMockContext(fake_payload);

            // Sign the fake payload
            (bytes memory _metadata, bytes memory readyMessage) = getVerifiedMessage(address(GARP), fake_payload);

            // Execute the fake payload
            GARP.processPacket(_metadata, readyMessage, bytes32(abi.encode(address(this))));

            // Compute max extractable balance:

            uint256 max_extract = 0;
            if (amplified) {
                // We need to count the total pool balance and then divide by 2.
                // This doesn't really work if there is more than 3 tokens though. :|
                uint256 vault_tokens = 0;
                while (true) {
                    address vault_tkn_1 = ICatalystV1Vault(vault)._tokenIndexing(vault_tokens);
                    uint256 token_weight_1 = ICatalystV1Vault(vault)._weight(vault_tkn_1);
                    if (vault_tkn_1 == address(0)) break;
                    vault_tokens += 1;
                    max_extract += Token(vault_tkn_1).balanceOf(vault) * token_weight_1 / 2;
                }
                address vault_tkn = ICatalystV1Vault(vault)._tokenIndexing(target_token_index);
                uint256 token_weight = ICatalystV1Vault(vault)._weight(vault_tkn);
                max_extract /= token_weight;
            } else {
                // Get the weight of the target token.
                uint256 weight_of_target_token = ICatalystV1Vault(vault)._weight(target_token);

                uint256 vault_tokens = 1;
                uint256 weightSum = weight_of_target_token;
                while (true) {
                    address vault_tkn = ICatalystV1Vault(vault)._tokenIndexing(vault_tokens);
                    if (vault_tkn == address(0)) break;
                    weightSum += ICatalystV1Vault(vault)._weight(vault_tkn);
                    vault_tokens += 1;
                }

                // How much can be extracted is based on which tokens is getting extracted and all associated weights.
                // Say that we have 3 tokens: weight 3, 1, 1 => Weight sum = 5. If we were to extract only token 1,
                // then the proportion would be 5/3 "halves". That is, we can half the balance ≈ 1.67 times.
                // That equates to 1/2**(5/3) = 31.5% of the vault is left or 68.5% of the vault has been extracted.
                // Because of the complexity assocaited, we allow a small error in the margin.

                uint256 extraction_ratio = 10**18 - 10**18 * 10**18 / uint256(FixedPointMathLib.powWad(
                    int256(2 * 10**18),
                    int256(weightSum * 10**18 / weight_of_target_token)
                ));
                max_extract = initial_target_token_balance * extraction_ratio / 10**18;
            }
            
            // Allow a certain margin of error.
            assertGe(max_extract * 10001/10000, Token(target_token).balanceOf(toAccount), "More than expected exploited");

            vm.revertTo(snapshot);
        }
    }
}

