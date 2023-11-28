// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "../../../src/ICatalystV1Vault.sol";
import "../../../src/CatalystVaultAmplified.sol";

import "../Invariant.t.sol";
import {TestSendAsset} from "../SendAsset.t.sol";
import {TestReceiveAsset} from "../ReceiveAsset.t.sol";
import { TestSendLiquidity } from "../SendLiquidity.t.sol";
import { TestReceiveLiquidity } from "../ReceiveLiquidity.t.sol";
import { TestFullLiquiditySwap } from "../FullLiquiditySwap.t.sol";
import { TestDepositArbitrageExploit } from "../non-exploits/DepositArbitrage.t.sol";
import "../non-exploits/CrossSwap.SwapWorthlessToken.t.sol";
import {Token} from "../../mocks/token.sol";

contract TestAmplifiedInvariant2 is TestInvariant, TestSendAsset, TestReceiveAsset, TestSwapWorthlessTokenCrossChain, TestReceiveLiquidity, TestSendLiquidity, TestFullLiquiditySwap, TestDepositArbitrageExploit  {

    address[] _vaults;

    function setUp() virtual override public {
        super.setUp();

        amplified = true;
        
        address[] memory assets = getTokens(3);
        uint256[] memory init_balances = new uint256[](3);
        init_balances[0] = 10 * 10**18; init_balances[1] = 100 * 10**18; init_balances[2] = 1000 * 10**18;
        uint256[] memory weights = new uint256[](3);
        weights[0] = 100; weights[1] = 10; weights[2] = 1;

        address vault1 = deployVault(assets, init_balances, weights, 10**18 / 4, 0);

        _vaults.push(vault1);

        assets = getTokens(2);
        init_balances = new uint256[](2);
        init_balances[0] = 100 * 10**18; init_balances[1] = 10 * 10**18;
        weights = new uint256[](2);
        weights[0] = 10; weights[1] = 100;

        address vault2 = deployVault(assets, init_balances, weights, 10**18 / 4, 0);

        _vaults.push(vault2);

    }
    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset) view override internal returns(uint256 amount) {
        return getLargestSwap(fromVault, toVault, fromAsset, toAsset, false);
    }

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset, bool securityLimit) view override internal returns(uint256 amount) {
        uint256 fromWeight = ICatalystV1Vault(fromVault)._weight(fromAsset);
        uint256 toWeight = ICatalystV1Vault(toVault)._weight(toAsset);

        if (securityLimit) {
            amount = Token(toAsset).balanceOf(toVault) * toWeight / fromWeight / 2;
        } else {
            amount = Token(toAsset).balanceOf(toVault) * toWeight / fromWeight;
        }
        uint256 amount2 = Token(fromAsset).balanceOf(address(this));
        if (amount2 < amount) amount = amount2;
    }
    
    function invariant(address[] memory vaults) view internal override returns(uint256 inv) {
        (uint256[] memory balances, uint256[] memory weights) = getBalances(vaults);

        int256 oneMinusAmp = CatalystVaultAmplified(vaults[0])._oneMinusAmp();

        balances = xProduct(balances, weights);

        balances = powerArray(balances, oneMinusAmp);

        inv = getSum(balances);
    }

    // Uses the invariant \sum (i · W)^(1-amp) / \sum (i_0 · W)^(1-amp) = constant for deposits and withdrawals.
    // TODO: Fix
    function strong_invariant(address vault) view internal override returns(uint256 inv) {
        address[] memory vaults = new address[](1);
        vaults[0] = vault;
        (uint256[] memory balances, uint256[] memory weights) = getBalances(vaults);

        // Get the number of tokens.
        uint256 numTokens = balances.length;

        int256 oneMinusAmp = CatalystVaultAmplified(vaults[0])._oneMinusAmp();

        uint256 balance0 = CatalystVaultAmplified(vault).computeBalance0();

        uint256 denum = balance0 * numTokens;

        balances = xProduct(balances, weights);

        balances = powerArray(balances, oneMinusAmp);

        inv = getSum(balances) / denum;
    }

    
    function getTestConfig() internal override view returns(address[] memory vaults) {
        return vaults = _vaults;
    }
}

