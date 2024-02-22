// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import { TestCommon } from "../TestCommon.t.sol";
import {Token} from "../mocks/token.sol";
import "../../src/ICatalystV1Vault.sol";

contract TestDeployVault is TestCommon {
    

    function test_deploy_3_token_volatile(uint16[3] memory weights_) external {
        vm.assume(weights_[0] > 0);
        vm.assume(weights_[1] > 0);
        vm.assume(weights_[2] > 0);
        address[] memory tokens = getTokens(3);

        uint256[] memory init_balances = new uint256[](3);
        init_balances[0] = 10000 * 10**18;
        init_balances[1] = 5000 * 10**18;
        init_balances[2] = 123123 * 10**18;

        uint256[] memory weights = new uint256[](3);
        weights[0] = uint256(weights_[0]);
        weights[1] = uint256(weights_[1]);
        weights[2] = uint256(weights_[2]);

        approveTokens(address(catFactory), tokens, init_balances);
        t_deploy_volatile(tokens, init_balances, weights);
    }

    function test_deploy_2_token_volatile(uint16[2] memory weights_) external {
        vm.assume(weights_[0] > 0);
        vm.assume(weights_[1] > 0);
        address[] memory tokens = getTokens(2);

        uint256[] memory init_balances = new uint256[](2);
        init_balances[0] = 10000 * 10**18;
        init_balances[1] = 5000 * 10**18;

        uint256[] memory weights = new uint256[](2);
        weights[0] = uint256(weights_[0]);
        weights[1] = uint256(weights_[1]);

        approveTokens(address(catFactory), tokens, init_balances);
        t_deploy_volatile(tokens, init_balances, weights);
    }

    function t_deploy_volatile(address[] memory tokens, uint256[] memory init_balances, uint256[] memory weights) internal {
        address vault = catFactory.deployVault(
            address(volatileTemplate),
            tokens,
            init_balances,
            weights,
            10**18,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

        verifyBalances(address(vault), tokens, init_balances);
        for (uint256 i = 0; i < tokens.length; ++i) {
            assertEq(
                ICatalystV1Vault(vault)._weight(tokens[i]),
                weights[i],
                "verifyBalances(...) failed"
            );
        }

        for (uint256 i = 0; i < tokens.length; ++i) {
            assertEq(
                ICatalystV1Vault(vault)._tokenIndexing(i),
                tokens[i],
                "verifyBalances(...) failed"
            );
        }

        assertEq(
            Token(vault).balanceOf(address(this)),
            10**18,
            "Unexpected pool tokens minted to pool creator"
        );
    }

    function test_deploy_3_token_amplified(uint16[3] memory weights_, uint64 amplificationPercentage) external {
        uint64 amplification = uint64(uint256(10**18) * uint256(amplificationPercentage) / uint256(type(uint64).max));
        vm.assume(amplification < 10**18);
        vm.assume(0 < amplification);
        vm.assume(weights_[0] > 0);
        vm.assume(weights_[1] > 0);
        vm.assume(weights_[2] > 0);
        address[] memory tokens = getTokens(3);

        uint256[] memory init_balances = new uint256[](3);
        init_balances[0] = 10000 * 10**18;
        init_balances[1] = 5000 * 10**18;
        init_balances[2] = 123123 * 10**18;

        uint256[] memory weights = new uint256[](3);
        weights[0] = uint256(weights_[0]);
        weights[1] = uint256(weights_[1]);
        weights[2] = uint256(weights_[2]);

        approveTokens(address(catFactory), tokens, init_balances);
        t_deploy_amplified(tokens, init_balances, weights, amplification);
    }

    function test_deploy_2_token_amplified(uint16[2] memory weights_, uint64 amplificationPercentage) external {
        uint64 amplification = uint64(uint256(10**18) * uint256(amplificationPercentage) / uint256(type(uint64).max));
        vm.assume(amplification < 10**18);
        vm.assume(0 < amplification);
        vm.assume(weights_[0] > 0);
        vm.assume(weights_[1] > 0);
        address[] memory tokens = getTokens(2);

        uint256[] memory init_balances = new uint256[](2);
        init_balances[0] = 10000 * 10**18;
        init_balances[1] = 5000 * 10**18;

        uint256[] memory weights = new uint256[](2);
        weights[0] = uint256(weights_[0]);
        weights[1] = uint256(weights_[1]);

        approveTokens(address(catFactory), tokens, init_balances);
        t_deploy_amplified(tokens, init_balances, weights, amplification);
    }

    function t_deploy_amplified(address[] memory tokens, uint256[] memory init_balances, uint256[] memory weights, uint64 amplification) internal {
        address vault = catFactory.deployVault(
            address(amplifiedTemplate),
            tokens,
            init_balances,
            weights,
            amplification,
            0,
            DEFAULT_POOL_NAME,
            DEFAULT_POOL_SYMBOL,
            address(CCI)
        );

        verifyBalances(address(vault), tokens, init_balances);
        for (uint256 i = 0; i < tokens.length; ++i) {
            assertEq(
                ICatalystV1Vault(vault)._weight(tokens[i]),
                weights[i],
                "verifyBalances(...) failed"
            );
        }

        for (uint256 i = 0; i < tokens.length; ++i) {
            assertEq(
                ICatalystV1Vault(vault)._tokenIndexing(i),
                tokens[i],
                "verifyBalances(...) failed"
            );
        }

        assertEq(
            Token(vault).balanceOf(address(this)),
            10**18,
            "Unexpected pool tokens minted to pool creator"
        );
    }
}

