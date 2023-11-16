// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "../../../src/ICatalystV1Vault.sol";
import "../../../src/utils/FixedPointMathLib.sol";

import "../Invariant.t.sol";
import "../LocalSwap/LocalSwap.t.sol";
import "../LocalSwap/LocalSwap.minout.t.sol";
import "../LocalSwap/LocalSwap.fees.t.sol";
import "../non-exploits/LocalSwap.SwapWorthlessToken.t.sol";
import "../Set/SetVaultFee.t.sol";
import "../Set/SetGovernanceFee.t.sol";
import "../Setup/Setup.t.sol";
import "../Setup/SetupFinish.t.sol";
import "../CrossChainInterfaceOnly.t.sol";
import "../TokenInterface.t.sol";
import "../Escrow.t.sol";
import "../Withdraw/WithdrawCompare.sol";
import "../Withdraw/WithdrawInvariant.sol";
import { TestCompareDepositWithWithdraw } from "../Deposit/DepositWithdrawCompare.t.sol";
import { TestSelfSwap } from "../SelfSwap.t.sol";
import {Token} from "../../mocks/token.sol";

contract TestVolatileInvariant is TestInvariant, TestLocalswap, TestCrossChainInterfaceOnly, TestLocalswapMinout, TestPoolTokenInterface, TestSetup, TestSetupFinish, TestSetVaultFee, TestSetGovernanceFee, TestLocalswapFees, TestSwapWorthlessTokenLocal, TestEscrow, TestWithdrawInvariant, TestWithdrawComparison, TestCompareDepositWithWithdraw, TestSelfSwap {

    address[] _vaults;

    function setUp() virtual override public {
        super.setUp();

        amplified = false;
        
        address[] memory assets = getTokens(3);
        uint256[] memory init_balances = new uint256[](3);
        init_balances[0] = 10 * 10**18; init_balances[1] = 100 * 10**18; init_balances[2] = 1000 * 10**18;
        uint256[] memory weights = new uint256[](3);
        weights[0] = 3; weights[1] = 2; weights[2] = 1;

        address vault1 = deployVault(assets, init_balances, weights, 10**18, 0);

        _vaults.push(vault1);

        assets = getTokens(2);
        init_balances = new uint256[](2);
        init_balances[0] = 1 * 10**18; init_balances[1] = 1 * 10**18;
        weights = new uint256[](2);
        weights[0] = 10; weights[1] = 10;

        address vault2 = deployVault(assets, init_balances, weights, 10**18, 0);

        _vaults.push(vault2);
    }

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset) view override internal returns(uint256 amount) {
        return getLargestSwap(fromVault, toVault, fromAsset, toAsset, false);
    }

    function getLargestSwap(address fromVault, address toVault, address fromAsset, address toAsset, bool securityLimit) view override internal returns(uint256 amount) {
        if (securityLimit) {
            amount = Token(fromAsset).balanceOf(fromVault) / 2;
        } else {
            amount = Token(fromAsset).balanceOf(fromVault) * 1000;
        }
        uint256 amount2 = Token(fromAsset).balanceOf(address(this));
        if (amount2 < amount) amount = amount2;
    }
    
    function invariant(address[] memory vaults) view internal override returns(uint256 inv) {
        (uint256[] memory balances, uint256[] memory weights) = getBalances(vaults);

        balances = logArray(balances);

        balances = xProduct(balances, weights);

        inv = getSum(balances);
    }

    // Uses the invariant \prod x^w / TS = constant for deposits and withdrawals.
    // It is rewritten as \sum ln(x) * w - ln(TS) = constant
    // TODO: Fix
    function strong_invariant(address vault) view internal override returns(uint256 inv) {
        address[] memory vaults = new address[](1);
        vaults[0] = vault;
        (uint256[] memory balances, uint256[] memory weights) = getBalances(vaults);

        uint256 vaultTokens = Token(vault).totalSupply();

        balances = logArray(balances);

        balances = xProduct(balances, weights);

        inv = getSum(balances) - uint256(FixedPointMathLib.lnWad(int256(vaultTokens)));
        
    }

    function getTestConfig() internal override view returns(address[] memory vaults) {
        return vaults = _vaults;
    }

    function getWithdrawPercentages(address vault, uint256[] memory withdraw_weights) internal override returns(uint256[] memory new_weights) {
        new_weights = new uint256[](withdraw_weights.length);
        // get weights
        uint256 progressiveWeightSum = 0;
        new_weights = new uint256[](withdraw_weights.length);
        for (uint256 i = withdraw_weights.length - 1; ;) {
            ICatalystV1Vault v = ICatalystV1Vault(vault);
            address tkn = v._tokenIndexing(i);
            uint256 ww = withdraw_weights[i];
            uint256 tw = v._weight(tkn);
            progressiveWeightSum += tw;
            new_weights[i] = ww * tw / progressiveWeightSum;
            if (i == 0) {
                break;
            }
            --i;
        }
    }
}

