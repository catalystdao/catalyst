// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "../../../src/ICatalystV1Vault.sol";
import "solady/utils/FixedPointMathLib.sol";

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
import "../Withdraw/WithdrawCompare.t.sol";
import "../Withdraw/WithdrawInvariant.t.sol";
import { TestCompareDepositWithWithdraw } from "../Deposit/DepositWithdrawCompare.t.sol";
import { TestWithdrawNothing } from "../Withdraw/WithdrawNothing.t.sol";
import { TestWithdrawUnbalanced } from "../Withdraw/WithdrawUnbalanced.t.sol";
import { TestSelfSwap } from "../SelfSwap.t.sol";
import { TestSetWeights } from "./SetWeights.t.sol";
import { TestVaultConnections } from "../VaultConnections.t.sol";
import { TestEvilRouterExploitVolatile } from "../non-exploits/EvilRoutor.Securitylimit.Volatile.t.sol";
import { TestSecurityLimitAssetSwap } from "../SecurityLimit.ReceiveAsset.t.sol";
import { TestSecurityLimitLiquiditySwap } from "../SecurityLimit.ReceiveLiquidity.t.sol";
import { TestWithdrawEverything } from "../Withdraw/WithdrawEverything.t.sol";
import {Token} from "../../mocks/token.sol";

contract TestVolatileInvariant is TestInvariant, TestLocalswap, TestCrossChainInterfaceOnly, TestLocalswapMinout, TestPoolTokenInterface, TestSetup, TestSetupFinish, TestSetVaultFee, TestSetGovernanceFee, TestSetWeights, TestLocalswapFees, TestSwapWorthlessTokenLocal, TestEscrow, TestWithdrawInvariant, TestWithdrawComparison,  TestCompareDepositWithWithdraw, TestWithdrawNothing, TestWithdrawUnbalanced, TestSelfSwap, TestVaultConnections, TestSecurityLimitAssetSwap, TestSecurityLimitLiquiditySwap, TestWithdrawEverything { //,TestEvilRouterExploitVolatile  {

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

    /**
     * @notice Compues adjusted withdrawal percentages
     * @dev Ideally we wouldn't have to do this but it hasn't been possible to find a solution to overcome the problem.
     * For simplicity, examine the withdrawal rations with 3 tokens. Let the withdrawal weights be WR_1, WR_2, and WR_3. (notice inverse order)
     * [WR_3 * w_3/(WR_3 * w_3), WR_3 * w_2 / (WR_2*w_2 + WR_3*w_3), WR_1*w_1 / (WR_1*w_1, WR_2*w_2 + WR_3*w_3)]
     * We want it to be [WR_3/WR_3, WR_2/(WR_2+WR_3), WR_1/(WR_1+WR_2+WR_3)]
     * For each of these indexes, to correct them we need to:
     * for 3 WR_3/WR_3 = WR_3 * w_3/(WR_3 * w_3) * (WR_3 * w_3) / (w_3 * WR_3)  (aka nothing)
     * for 2: WR_2/(WR_2+WR_3) = b * w_2 / (WR_2*w_2 + WR_3*w_3) * (WR_2*w_2 + WR_3*w_3)/(w_2 * (WR_2+WR_3))
     * for 1: WR_1/(WR_1+WR_2+WR_3) = WR_1*w_1 / (WR_1*w_1, WR_2*w_2 + WR_3*w_3) * (WR_1*w_1, WR_2*w_2 + WR_3*w_3)/(w_1 * (WR_1+WR_2+WR_3))
     * That implies, generally we need to multiply the i'th index by: \sum_(j >= i) (WR_j * w_j) / (w_i * \sum_(j >= i) WR_j)
     * This is fairly complicated and kindof expensive. 
     */
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
            progressiveWeightSum += tw * ww;
            new_weights[i] = ww * tw * 10**18 / progressiveWeightSum;
            if (i == 0) {
                break;
            }
            --i;
        }
    }
}

