//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {ICatalystMathLibVol} from "./interfaces/ICatalystMathLibVol.sol";
import "../FixedPointMathLib.sol";
import "../interfaces/ICatalystV1PoolDerived.sol";
import "../interfaces/ICatalystV1PoolState.sol";
import "../SwapPoolVolatile.sol";

/**
 * @title Catalyst: Volatile mathematics implementation
 * @author Catalyst Labs
 * @notice This contract is not optimised for on-chain calls and serves to aid in off-chain quering.
 */
contract CatalystMathVol is ICatalystMathLibVol {
    
    /// @notice Helper function which returns the true weight. If weights are being adjusted, the pure vault weights might lie.
    function getTrueWeight(address vault, address asset) public view returns(uint256) {
        // First, lets check if we actually needs to do any adjustments:
        uint256 adjTarget = CatalystSwapPoolVolatile(vault)._adjustmentTarget();

        uint256 currentWeight = CatalystSwapPoolVolatile(vault)._weight(asset);
        
        if (adjTarget == 0) 
            return currentWeight; // Great, we don't need to do any adjustments:

        // We need to do the adjustment. Fetch relevant variables.
        uint256 targetWeight = CatalystSwapPoolVolatile(vault)._weight(asset);
        uint256 lastModification = CatalystSwapPoolVolatile(vault)._lastModificationTime();

        // If the current time is past the adjustment, we should return the final weights
        if (block.timestamp >= adjTarget) 
            return targetWeight;

        if (targetWeight > currentWeight) {
            // if the weights are increased then targetWeight - currentWeight > 0.
            // Add the change to the current weight.
            uint256 newWeight = currentWeight + (
                (targetWeight - currentWeight) * (block.timestamp - lastModification)
            ) / (adjTarget - lastModification);
            return newWeight;
        } else {
            // if the weights are decreased then targetWeight - currentWeight < 0.
            // Subtract the change from the current weights.
            uint256 newWeight = currentWeight - (
                (currentWeight - targetWeight) * (block.timestamp - lastModification)
            ) / (adjTarget - lastModification);
            return newWeight;
        }
    }

    /// @notice Helper function which returns the amount after fee.
    function calcFee(address vault, uint256 amount) public view returns(uint256) {
        uint256 fee = CatalystSwapPoolVolatile(vault)._poolFee();

        return FixedPointMathLib.mulWadDown(amount, FixedPointMathLib.WAD - fee);
    }
    
    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{A}^{A+x} W/w dw = W ln((A+x)/A)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current vault balance of the x token.
     * @param W The weight of the x token.
     * @return uint256 Group-specific units (units are **always** WAD).
     */
    function calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W
    ) public pure returns (uint256) {
        // Notice, A + in and A are not WAD but divWadDown is used anyway.
        // That is because lnWad requires a scaled number.
        return W * uint256(FixedPointMathLib.lnWad(int256(FixedPointMathLib.divWadDown(A + input, A))));    // int256 casting is safe. If overflows, it returns negative. lnWad fails on negative numbers. If the vault balance is high, this is unlikely.
    }

    /**
     * @notice Solves the equation U = \int_{B-y}^{B} W/w dw for y = B · (1 - exp(-U/W))
     * The value is returned as output token. (not WAD)
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always multiplied by WAD, the function
     * should be treated as mathematically *native*.
     * @param U Incoming group specific units.
     * @param B The current vault balance of the y token.
     * @param W The weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W
    ) public pure returns (uint256) {
        return FixedPointMathLib.mulWadDown(
            B,
            FixedPointMathLib.WAD - uint256(FixedPointMathLib.expWad(-int256(U / W)))   // int256 casting is initially not safe. If overflow, the equation becomes: 1 - exp(U/W) => exp(U/W) > 1. In this case, Solidity's built-in safe math protection catches the overflow.
        );
    }

    /**
     * @notice Solves the equation
     *     \int_{A}^{A+x} W_a/w dw = \int_{B-y}^{B} W_b/w dw for y = B · (1 - ((A+x)/A)^(-W_a/W_b))
     * @dev All input amounts should be the raw numbers and not WAD. Is computed through calcPriceCurveLimit(calcPriceCurveArea).
     * @param input The input amount.
     * @param A The current vault balance of the x token.
     * @param B The current vault balance of the y token.
     * @param W_A The weight of the x token.
     * @param W_B TThe weight of the y token.
     * @return uint256 Output denominated in output token.
     */
    function calcCombinedPriceCurves(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B
    ) public pure returns (uint256) {
        // uint256 U = FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
        //     int256(FixedPointMathLib.divWadDown(A + input, A)),
        //     int256(FixedPointMathLib.divWadDown(W_A, W_B))
        //)); 
        // return (B * U) / FixedPointMathLib.WAD;
        return calcPriceCurveLimit(calcPriceCurveArea(input, A, W_A), B, W_B);
    }

    /**
     * @notice Solves the generalised swap integral.
     * @dev Based on _calcPriceCurveLimit but the multiplication by the
     * specific token is never done.
     * @param U Input units.
     * @param W The generalised weights.
     * @return uint256 Output denominated in vault share.
     */
    function calcPriceCurveLimitShare(
        uint256 U,
        uint256 W
    ) public pure returns (uint256) {
        // Compute the non vault ownership share. (1 - vault ownership share)
        uint256 npos = uint256(FixedPointMathLib.expWad(-int256(U / W)));   // int256 casting is initially not safe. If overflow, the equation becomes: exp(U/W). In this case, when subtracted from 1 (later), Solidity's built-in safe math protection catches the overflow since exp(U/W) > 1.
        
        // Compute the vault owner share before liquidity has been added.
        // (solve share = pt/(PT+pt) for pt.)
        return FixedPointMathLib.divWadDown(FixedPointMathLib.WAD - npos, npos);
    }

    // To compute the result of a cross-chain swap, find the mathematical contract for each chain which you want to swap to.
    // Then find calcSendAsset and calcReceiveAsset.
    // Compute the intermediate value, units, with calcSendAsset:
    // U = calcSendAsset(...) on the sending chain
    // Then compute the output as:
    // quote = calcReceiveAsset(..., U) on the target chain.

    /**
     * @notice Computes the exchange of assets to units. This is the first part of a swap.
     * @dev Returns 0 if from is not a token in the vault
     * @param vault The vault address to examine.
     * @param fromAsset The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @return uint256 Group-specific units.
     */
    function calcSendAsset(
        address vault,
        address fromAsset,
        uint256 amount
    ) external view override returns (uint256) {
        // A high => fewer units returned. Do not subtract the escrow amount
        uint256 A = calcFee(vault, ERC20(fromAsset).balanceOf(vault));
        uint256 W = getTrueWeight(vault, fromAsset);

        // If a token is not part of the vault, W is 0. This returns 0 by
        // multiplication with 0.
        return calcPriceCurveArea(amount, A, W);
    }

    /**
     * @notice Computes the exchange of units to assets. This is the second and last part of a swap.
     * @dev Reverts if to is not a token in the vault
     * @param vault The vault address to examine.
     * @param toAsset The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveAsset(
        address vault,
        address toAsset,
        uint256 U
    ) external view override returns (uint256) {
        // B low => fewer tokens returned. Subtract the escrow amount to decrease the balance.
        uint256 B = ERC20(toAsset).balanceOf(vault) - CatalystSwapPoolVolatile(vault)._escrowedTokens(toAsset);
        uint256 W = getTrueWeight(vault, toAsset);

        // If someone were to purchase a token which is not part of the vault on setup
        // they would just add value to the vault. We don't care about it.
        // However, it will revert since the solved integral contains U/W and when
        // W = 0 then U/W returns division by 0 error.
        return calcPriceCurveLimit(U, B, W);
    }

    /**
     * @notice Computes the output of localSwap.
     * @dev If the vault weights of the 2 tokens are equal, a very simple curve is used.
     * If from or to is not part of the vault, the swap will either return 0 or revert.
     * If both from and to are not part of the vault, the swap can actually return a positive value.
     * @param vault The vault address to examine.
     * @param fromAsset The address of the token to sell.
     * @param toAsset The address of the token to buy.
     * @param amount The amount of from token to sell for to token.
     * @return uint256 Output denominated in toAsset.
     */
    function calcLocalSwap(
        address vault,
        address fromAsset,
        address toAsset,
        uint256 amount
    ) external view override returns (uint256) {
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - CatalystSwapPoolVolatile(vault)._escrowedTokens(toAsset);
        uint256 W_A = getTrueWeight(vault, fromAsset);
        uint256 W_B = getTrueWeight(vault, toAsset);

        // The swap equation simplifies to the ordinary constant product if the
        // token weights are equal.
        if (W_A == W_B)
            // Saves gas and is exact.
            // NOTE: If W_A == 0 and W_B == 0 => W_A == W_B => The calculation will not fail.
            // This is not a problem, since W_B != 0 for assets contained in the vault, and hence a 0-weighted asset 
            // (i.e. not contained in the vault) cannot be used to extract an asset contained in the vault.
            return (B * amount) / (A + amount);

        // If either token doesn't exist, their weight is 0.
        // Then powWad returns 1 which is subtracted from 1 => returns 0.
        return calcCombinedPriceCurves(amount, A, B, W_A, W_B);
    }

    /**
    * @notice Computes part of the mid price. Requires calcCurrentPriceTo to convert into a pairwise price.
    * @param vault The vault address to examine.
    * @param fromAsset The address of the token to sell.
    */
    function calcAsyncPriceFrom(
        address vault,
        address fromAsset
    ) public view returns (uint256) {
        uint256 fromBalance = ERC20(fromAsset).balanceOf(vault);
        uint256 W_from = getTrueWeight(vault, fromAsset);
        if (W_from == 0) return 0;

        return (fromBalance * 10**18)/W_from;
    }

    /**
    * @notice Computes a pairwise mid price. Requires input from calcAsyncPriceFrom.
    * @param vault The vault address to examine.
    * @param toAsset The address of the token to buy.
    * @param calcAsyncPriceFromQuote The output of calcAsyncPriceFrom.
    */
    function calcCurrentPriceTo(
        address vault,
        address toAsset,
        uint256 calcAsyncPriceFromQuote
    ) public view returns (uint256) {
        uint256 toBalance = ERC20(toAsset).balanceOf(vault) - CatalystSwapPoolVolatile(vault)._escrowedTokens(toAsset);
        uint256 W_to = getTrueWeight(vault, toAsset);
        if ((calcAsyncPriceFromQuote == 0) || (W_to == 0)) return 0;

        return (toBalance * 10**18)/(W_to * calcAsyncPriceFromQuote);
    }

    /**
    * @notice Computes the current mid price. This is the current marginal price between the 2 assets.
    * @dev The mid price cannot be traded on, since the fees acts as the spread.
    * @param vault The vault address to examine.
    * @param fromAsset The address of the token to sell.
    * @param toAsset The address of the token to buy.
    * @return uint256 Output denominated in toAsset.
    */
    function calcCurrentPrice(
        address vault,
        address fromAsset,
        address toAsset
    ) external view returns (uint256) {
        uint256 calcAsyncPriceFromQuote = calcAsyncPriceFrom(vault, fromAsset);

        return calcCurrentPriceTo(vault, toAsset, calcAsyncPriceFromQuote);
    }
}
