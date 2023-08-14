// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

import {Constants} from '../libraries/Constants.sol';
import {RouterImmutables} from '../base/RouterImmutables.sol';
import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {Payments} from './Payments.sol';
import {ICatalystV1Vault} from '../../ICatalystV1Vault.sol';
import {BytesLib} from './BytesLib.sol';
import {ICatalystV1Structs} from '../../interfaces/ICatalystV1VaultState.sol';

/// @title Catalyst Exchange Wrapper
/// @notice Wraps the Catalyst exchange calls
abstract contract CatalystExchange is RouterImmutables, ICatalystV1Structs {
    using BytesLib for bytes;

    /**
     * @notice A swap between 2 assets which both are inside the vault. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of _toAsset the user wants.
     */
    function localSwap(
        address vault,
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(fromAsset).balanceOf(address(this)) : amount;

        ERC20(fromAsset).approve(vault, amount);

        ICatalystV1Vault(vault).localSwap(
            fromAsset,
            toAsset,
            amount,
            minOut
        );
    }

    function sendAsset(
        address vault,
        RouteDescription memory routeDescription,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        uint256 gas,
        bytes calldata calldata_
    ) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(fromAsset).balanceOf(address(this)) : amount;
        gas = gas == Constants.CONTRACT_BALANCE ? address(this).balance : gas;

        ERC20(fromAsset).approve(vault, amount);

        ICatalystV1Vault(vault).sendAsset{value: gas}(
            routeDescription,
            fromAsset,
            toAssetIndex,
            amount,
            minOut,
            fallbackUser,
            calldata_
        );
    }

    function sendLiquidity(
        address vault,
        RouteDescription memory routeDescription,
        uint256 vaultTokens,
        uint256[2] memory minOut,
        address fallbackUser,
        uint256 gas,
        bytes calldata calldata_
    ) internal {
        vaultTokens = vaultTokens == Constants.CONTRACT_BALANCE ? ERC20(vault).balanceOf(address(this)) : vaultTokens;
        gas = gas == Constants.CONTRACT_BALANCE ? address(this).balance : gas;

        ICatalystV1Vault(vault).sendLiquidity{value: gas}(
            routeDescription,
            vaultTokens,
            minOut,
            fallbackUser,
            calldata_
        );
    }

    /**
     * @notice Deposits a user configurable amount of tokens.
     * @dev Requires approvals for all tokens within the vault.
     * Volatile: It is advised that the deposit matches the vault's %token distribution.
     * Amplified: It is advised that the deposit is as close to 1,1,... as possible.
     *            Otherwise between 1,1,... and the vault's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of vault tokens to be minted.
     */
    function depositMixed(address vault, address[] calldata tokens, uint256[] memory tokenAmounts, uint256 minOut) internal {
        uint256 numberOfTokens = tokenAmounts.length;
        for (uint256 it = 0; it < numberOfTokens; ++it) {
            uint256 tknAmount = tokenAmounts[it];
            if (tknAmount == Constants.CONTRACT_BALANCE) tknAmount = ERC20(tokens[it]).balanceOf(address(this));
            tokenAmounts[it] = tknAmount;

            ERC20(tokens[it]).approve(vault, tknAmount);
        }

        ICatalystV1Vault(vault).depositMixed(tokenAmounts, minOut);
    }

    /**
     * @notice Burns baseAmount and releases the symmetrical share
     * of tokens to the burner. This doesn't change the vault price.
     * @param amount The number of vault tokens to burn.
     */
    function withdrawAll(address vault, uint256 amount, uint256[] calldata minOut) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(vault).balanceOf(address(this)) : amount;

        ICatalystV1Vault(vault).withdrawAll(amount, minOut);
    }

    /**
     * @notice Burns vaultTokens and release a token distribution which can be set by the user.
     * @dev Requires approvals for all tokens within the vault.
     * Volatile: It is advised that the deposit matches the vault's %token distribution.
     * Amplified: It is advised that the deposit matches the vault's %token distribution.
     *            Otherwise it should be weighted towards the tokens the vault has more of.
     * @param amount The number of vault tokens to withdraw
     * @param withdrawRatio The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is X64
     * @param minOut The minimum number of tokens minted.
     */
    function withdrawMixed(
        address vault,
        uint256 amount,
        uint256[] calldata withdrawRatio,
        uint256[] calldata minOut
    ) internal {
        amount = amount == Constants.CONTRACT_BALANCE ? ERC20(vault).balanceOf(address(this)) : amount;
        
        ICatalystV1Vault(vault).withdrawMixed(amount, withdrawRatio, minOut);
    }
}