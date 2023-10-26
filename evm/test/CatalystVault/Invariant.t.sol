// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "../TestCommon.t.sol";
import "src/ICatalystV1Vault.sol";
import "src/utils/FixedPointMathLib.sol";
import {Token} from "../mocks/token.sol";

abstract contract TestInvariant is TestCommon {
    function getNumberOfAssets(address[] memory vaults) view internal returns(uint256 numAssets) {
        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            uint256 j = 0;
            while (true) {
                address token = ICatalystV1Vault(vault)._tokenIndexing(j);
                if (token == address(0)) break;
                ++numAssets;
                ++j;
            }
        }
    }

    function getBalances(address[] memory vaults) view internal returns(uint256[] memory balances, uint256[] memory weights) {
        uint256 numAssets = getNumberOfAssets(vaults);
        balances = new uint256[](numAssets);
        weights = new uint256[](numAssets);

        uint256 counter = 0;
        for (uint256 i = 0; i < vaults.length; ++i) {
            address vault = vaults[i];
            uint256 j = 0;
            while (true) {
                address token = ICatalystV1Vault(vault)._tokenIndexing(j);
                if (token == address(0)) break;
                balances[counter] = Token(token).balanceOf(vault);
                weights[counter] = ICatalystV1Vault(vault)._weight(token);
                ++counter;
                ++j;
            }
        }
    }

    function logArray(uint256[] memory x) pure internal returns(uint256[] memory y) {
        y = new uint256[](x.length);
        for (uint256 i = 0; i < x.length; ++i) {
            y[i] = uint256(FixedPointMathLib.lnWad(int256(x[i] * FixedPointMathLib.WAD)));
        }
    }

    function getSum(uint256[] memory values) pure internal returns(uint256 sum) {
        for (uint256 i = 0; i < values.length; ++i) {
            sum += values[i];
        }
    }

    function xProduct(uint256[] memory a, uint256[] memory b) pure internal returns(uint256[] memory c) {
        require(a.length == b.length, "Cannot cross-multiply if lengths are different");
        c = new uint256[](a.length);
        for (uint256 i = 0; i < a.length; ++i) {
            c[i] = a[i] * b[i];
        }
    }

    function powerArray(uint256[] memory x, int256 power) pure internal returns(uint256[] memory y) {
        y = new uint256[](x.length);
        for (uint256 i = 0; i < x.length; ++i) {
            if (x[i] == 0) {
                y[i] = 0;
                continue;
            }
            y[i] = uint256(FixedPointMathLib.powWad(int256(x[i]), power));
        }
    }
}

