// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

import {Constants} from '../libraries/Constants.sol';
import {RouterImmutables} from '../base/RouterImmutables.sol';
import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {Payments} from './Payments.sol';
import {ICatalystV1Pool} from '../../ICatalystV1Pool.sol';
import {BytesLib} from './BytesLib.sol';

/// @title Catalyst Exchange Wrapper
/// @notice Wraps the Catalyst exchange calls
abstract contract CatalystExchange is RouterImmutables {
    using BytesLib for bytes;

    /**
     * @notice A swap between 2 assets which both are inside the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of _toAsset the user wants.
     */
    function localswap(
        address pool,
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(fromAsset).balanceOf(address(this)) : amount;

        ERC20(fromAsset).approve(pool, amount);

        ICatalystV1Pool(pool).localswap(
            fromAsset,
            toAsset,
            amount,
            minOut
        );
    }

    function sendSwap(
        address pool,
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        bytes calldata calldata_
    ) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(fromAsset).balanceOf(address(this)) : amount;

        ERC20(fromAsset).approve(pool, amount);

        ICatalystV1Pool(pool).sendSwap(
            channelId,
            targetPool,
            targetUser,
            fromAsset,
            toAssetIndex,
            amount,
            minOut,
            fallbackUser,
            calldata_
        );
    }

    /**
     * @notice Deposits a user configurable amount of tokens.
     * @dev Requires approvals for all tokens within the pool.
     * Volatile: It is advised that the deposit matches the pool's %token distribution.
     * Amplified: It is advised that the deposit is as close to 1,1,... as possible.
     *            Otherwise between 1,1,... and the pool's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of pool tokens to be minted.
     */
    function depositMixed(address pool, address[] memory tokens, uint256[] memory tokenAmounts, uint256 minOut) internal {
        uint256 numberOfTokens = tokenAmounts.length;
        for (uint256 it = 0; it < numberOfTokens; ++it) {
            uint256 tknAmount = tokenAmounts[it];
            if (tknAmount == Constants.CONTRACT_BALANCE) tknAmount = ERC20(tokens[it]).balanceOf(address(this));

            ERC20(tokens[it]).approve(pool, tknAmount);
        }

        ICatalystV1Pool(pool).depositMixed(tokenAmounts, minOut);
    }

    /**
     * @notice Burns baseAmount and releases the symmetrical share
     * of tokens to the burner. This doesn't change the pool price.
     * @param amount The number of pool tokens to burn.
     */
    function withdrawAll(address pool, uint256 amount, uint256[] memory minOut) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(pool).balanceOf(address(this)) : amount;

        ICatalystV1Pool(pool).withdrawAll(amount, minOut);
    }

    /**
     * @notice Burns poolTokens and release a token distribution which can be set by the user.
     * @dev Requires approvals for all tokens within the pool.
     * Volatile: It is advised that the deposit matches the pool's %token distribution.
     * Amplified: It is advised that the deposit matches the pool's %token distribution.
     *            Otherwise it should be weighted towards the tokens the pool has more of.
     * @param amount The number of pool tokens to withdraw
     * @param withdrawRatio The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is X64
     * @param minOut The minimum number of tokens minted.
     */
    function withdrawMixed(
        address pool,
        uint256 amount,
        uint256[] memory withdrawRatio,
        uint256[] memory minOut
    ) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(pool).balanceOf(address(this)) : amount;
        
        ICatalystV1Pool(pool).withdrawMixed(amount, withdrawRatio, minOut);
    }

    

}