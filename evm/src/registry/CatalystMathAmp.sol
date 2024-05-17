//SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import {ERC20} from 'solady/tokens/ERC20.sol';
import {ICatalystMathLibAmp} from "./interfaces/ICatalystMathLibAmp.sol";
import "solady/utils/FixedPointMathLib.sol";
import "../interfaces/ICatalystV1VaultDerived.sol";
import "../interfaces/ICatalystV1VaultState.sol";
import "../CatalystVaultAmplified.sol";
import "../IntegralsAmplified.sol";

/**
 * @title Catalyst: Amplified mathematics implementation
 * @author Catalyst Labs Inc.
 * @notice This contract is not optimised for on-chain calls and serves to aid in off-chain quering.
 */
contract CatalystMathAmp is IntegralsAmplified, ICatalystMathLibAmp {

    /**
     * @dev  When the swap is a very small size of the vault, the
     * swaps returns slightly more. To counteract this, an additional 
     * fee slightly larger than the error is added. The below 
     * constants determines when this fee is added and the size.
     */
    uint256 constant public SMALL_SWAP_RATIO = 1e12;
    uint256 constant public SMALL_SWAP_RETURN = 95e16;
    
    /**
     * @notice Helper function which returns the true amplification. If amp is being adjusted, the pure vault amp might be inaccurate.
     * @dev This function is unused.
     * If amplification is being changed, the amplification read directly from the vaults are only updated when they are needed. (swaps, balance changes, etc)
     * This function implements the amp change logic (almost exactly), such that one can read the amplifications if one were to execute a balance change.
     * @param vault Address of the vault to fetch the amp for.
     * @return uint256 Returns the (estimated) true amp.
     */
    function getTrueAmp(address vault) public view returns(int256) {
        // First, lets check if we actually needs to do any adjustments:
        uint256 adjTarget = CatalystVaultAmplified(vault)._adjustmentTarget();

        int256 currentAmplification = CatalystVaultAmplified(vault)._oneMinusAmp();

        if (adjTarget == 0) 
            return currentAmplification; // Great, we don't need to do any adjustments:

        // We need to do the adjustment. Fetch relevant variables.
        int256 targetAmplification = CatalystVaultAmplified(vault)._targetAmplification();
        uint256 lastModification = CatalystVaultAmplified(vault)._lastModificationTime();

        // If the current time is past the adjustment, we should return the final amplification.
        if (block.timestamp >= adjTarget) 
            return targetAmplification;
        

        unchecked {
            // Lets check each mathematical computation one by one.
            // First part is (targetAmplification - currentAmplification). We know that targetAmplification + currentAmplification < 2e18
            // => |targetAmplification - currentAmplification| < 2e18.

            // int256(block.timestamp - lastModification), it is fair to assume that block.timestamp < 2**64. Thus
            // block.timestamp - lastModification < block.timestamp < 2**64

            // |targetAmplification - currentAmplification| * (block.timestamp - lastModification) < 2*10**18 * 2**64  < 2**87 (no overflow)

            // dividing by int256(adjTarget - lastModification) reduces the number. If adjTarget = lastModification (division by 0)
            // => This function has been called before. Thus it must be that lastModification = block.timestamp. But that cannot be the case
            // since block.timestamp >= adjTarget => adjTarget = 0.

            // We know that int256(block.timestamp - lastModification) / int256(adjTarget - lastModification) < 1, since
            // adjTarget > block.timestamp. So int256(block.timestamp - lastModification) / int256(adjTarget - lastModification) *
            // |targetAmplification - currentAmplification| < 1 * 2**64.

            // Add the change to the current amp.
            return currentAmplification + (
                (targetAmplification - currentAmplification) * int256(block.timestamp - lastModification)  // timestamp is largest but small relative to int256.
            ) / int256(adjTarget - lastModification);   // adjTarget is bounded by block.timestap + 1 year
        }
        
    }

    /** 
     * @notice Helper function which returns the amount after fee.
     * @dev The fee is taken from the input amount.
     * @param vault Vault to read vault fee.
     * @param amount Input swap amount
     * @return uint256 Input amount after vault fee.
     */
    function calcFee(address vault, uint256 amount) public view returns(uint256) {
        uint256 fee = CatalystVaultAmplified(vault)._vaultFee();

        return FixedPointMathLib.mulWad(amount, FixedPointMathLib.WAD - fee);
    }
    
    /**
     * @notice Computes the integral \int_{wA}^{wA+wx} 1/w^k · (1-k) dw
     *     = (wA + wx)^(1-k) - wA^(1-k)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input Input amount.
     * @param A Current vault balance of the x token.
     * @param W Weight of the x token.
     * @param oneMinusAmp Amplification as (1-k)
     * @return uint256 Units (units are **always** WAD).
     */
    function calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W,
        int256 oneMinusAmp
    ) external pure returns (uint256) {
        return _calcPriceCurveArea(input, A, W, oneMinusAmp);
    }

    /**
     * @notice Solves the equation U = \int_{wA-_wy}^{wA} W/w^k · (1-k) dw for y
     *     = B · (1 - (
     *             (wB^(1-k) - U) / (wB^(1-k))
     *         )^(1/(1-k))
     *     )
     * The value is returned as output token. (not WAD)
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always multiplied by WAD, the function
     * should be treated as mathematically *native*.
     * @param U Incoming vault specific units.
     * @param B Current vault balance of the y token.
     * @param W Weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W,
        int256 oneMinusAmp
    ) public pure returns (uint256) {
        return _calcPriceCurveLimit(U, B, W, oneMinusAmp);
    }

    /**
     * @notice !Unused! Solves the equation
     *     \int_{wA}^{wA + wx} 1/w^k · (1-k) dw = \int_{wB-wy}^{wB} 1/w^k · (1-k) dw for y
     *         => out = B · (1 - (
     *                 (wB^(1-k) - (wA+wx)^(1-k) - wA^(1-k)) / (wB^(1-k))
     *             )^(1/(1-k))
     *         )
     *
     * Alternatively, the integral can be computed through:
     * _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A, amp), B, W_B, amp).
     * @dev All input amounts should be the raw numbers and not WAD.
     * @param input The input amount.
     * @param A Current vault balance of the _in token.
     * @param B Current vault balance of the _out token.
     * @param W_A Vault weight of the _in token.
     * @param W_B Vault weight of the _out token.
     * @param oneMinusAmp Amplification as (1-k).
     * @return uint256 Output denominated in output token.
     */
    function calcCombinedPriceCurves(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B,
        int256 oneMinusAmp
    ) public pure returns (uint256) {
        return _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A, oneMinusAmp), B, W_B, oneMinusAmp);
    }

    /**
     * @notice Converts units into vault tokens with the below formula
     *      pt = PT · (((N · wa_0^(1-k) + U)/(N · wa_0^(1-k))^(1/(1-k)) - 1)
     * @dev The function leaves a lot of computation to the external implementation. This is done to avoid recomputing values several times.
     * @param U Number of units to convert into vault tokens.
     * @param ts Current vault token supply. The escrowed vault tokens should not be added, since the function then returns more.
     * @param it_times_walpha_amped wa_0^(1-k)
     * @param oneMinusAmpInverse Vault amplification as 1/(1-k)
     * @return uint256 Output denominated in vault tokens.
     */
    function calcPriceCurveLimitShare(uint256 U, uint256 ts, uint256 it_times_walpha_amped, int256 oneMinusAmpInverse) external pure returns (uint256) {
        return _calcPriceCurveLimitShare(U, ts, it_times_walpha_amped, oneMinusAmpInverse);
    }

    // To compute the result of a cross-chain swap, find the mathematical contract for each chain which you want to swap to.
    // Then find calcSendAsset and calcReceiveAsset.
    // Compute the intermediate value, units, with calcSendAsset:
    // U = calcSendAsset(...) on the sending chain
    // Then compute the output as:
    // quote = calcReceiveAsset(..., U) on the target chain.

    /**
     * @notice Computes the exchange of assets to units. This is the first part of a swap.
     * @dev Reverts if fromAsset is not in the vault.
     * @param vault Vault address to examine.
     * @param fromAsset Address of the token to sell.
     * @param amount Amount of from token to sell.
     * @return uint256 Units.
     */
    function calcSendAsset(
        address vault,
        address fromAsset,
        uint256 amount
    ) external view override returns (uint256) {
        // A high => fewer units returned. Do not subtract the escrow amount
        uint256 A = calcFee(vault, ERC20(fromAsset).balanceOf(vault));
        uint256 W = CatalystVaultAmplified(vault)._weight(fromAsset);

        // If 'fromAsset' is not part of the vault (i.e. W is 0) or if 'amount' and 
        // the vault asset balance (i.e. 'A') are both 0 this will revert, since 0**p is 
        // implemented as exp(ln(0) * p) and ln(0) is undefined.
        uint256 U = _calcPriceCurveArea(amount, A, W, getTrueAmp(vault));

        // If the swap is a very small portion of the vault
        // Add an additional fee. This covers mathematical errors.
        unchecked { //SMALL_SWAP_RATIO is not zero, and if U * SMALL_SWAP_RETURN overflows, less is returned to the user
            if (A/SMALL_SWAP_RATIO >= amount) return U * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;
        }
        
        return U;
    }

    /**
     * @notice Computes the exchange of units to assets. This is the second and last part of a swap.
     * @dev Reverts if toAsset is not in the vault.
     * @param vault Vault address to examine.
     * @param toAsset Address of the token to buy.
     * @return uint256 tokens.
     */
    function calcReceiveAsset(
        address vault,
        address toAsset,
        uint256 U
    ) external view override returns (uint256) {
        // B low => fewer tokens returned. Subtract the escrow amount to decrease the balance.
        uint256 B = ERC20(toAsset).balanceOf(vault) - CatalystVaultAmplified(vault)._escrowedTokens(toAsset);
        uint256 W = CatalystVaultAmplified(vault)._weight(toAsset);

        // If someone were to purchase a token which is not part of the vault on setup
        // they would just add value to the vault. We don't care about it.
        // However, it will revert since the solved integral contains U/W and when
        // W = 0 then U/W returns division by 0 error.
        return _calcPriceCurveLimit(U, B, W, getTrueAmp(vault));
    }

    /**
     * @notice Computes the output of localSwap.
     * @dev Reverts if either from or to is not in the vault,
     * or if the vault 'fromAsset' balance and 'amount' are both 0.
     * @param vault Vault address to examine.
     * @param fromAsset Address of the token to sell.
     * @param toAsset Address of the token to buy.
     * @param amount Amount of from token to sell for to token.
     * @return uint256 Output denominated in toAsset.
     */
    function calcLocalSwap(
        address vault,
        address fromAsset,
        address toAsset,
        uint256 amount
    ) external view override returns (uint256) {
        uint256 A = ERC20(fromAsset).balanceOf(vault);
        uint256 B = ERC20(toAsset).balanceOf(vault) - CatalystVaultAmplified(vault)._escrowedTokens(toAsset);
        uint256 W_A = CatalystVaultAmplified(vault)._weight(fromAsset);
        uint256 W_B = CatalystVaultAmplified(vault)._weight(toAsset);
        int256 oneMinusAmp = getTrueAmp(vault);

        uint256 output = _calcCombinedPriceCurves(amount, A, B, W_A, W_B, oneMinusAmp);

        // If the swap is a very small portion of the vault
        // Add an additional fee. This covers mathematical errors.
        unchecked { //SMALL_SWAP_RATIO is not zero, and if output * SMALL_SWAP_RETURN overflows, less is returned to the user
            if (A/SMALL_SWAP_RATIO >= amount) return output * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;
        }

        return output;
    }

    //* Mid prices and infinitesimal trades.
    // The mid price is current price in the vault. It is a single point on the combined price curve
    // of a pair. As a result, it can never be traded on. Furthermore, fees result in a spread 
    // on both sides of the mid price.
    // The mid price, z, should be used to compute price impact. Given an input, x, and an output, y,
    // the trade price is y/x. The difference between y/x and z:  1 - (y/x)/z is the price impact.

    /**
    * @notice Computes part of the mid price. calcCurrentPriceTo can be used to compute the pairwise price.
    * @dev Alternativly, dividing calcAsyncPriceFrom by another calcAsyncPriceFrom, results in the pairwise price.
    * @param vault Vault address to examine.
    * @param fromAsset Address of the token to sell.
    */
    function calcAsyncPriceFrom(
        address vault,
        address fromAsset
    ) public view returns (uint256) {
        uint256 fromBalance = ERC20(fromAsset).balanceOf(vault);
        uint256 W_from = CatalystVaultAmplified(vault)._weight(fromAsset);
        int256 oneMinusAmp = getTrueAmp(vault);
        if (W_from == 0) return 0;

        return uint256(FixedPointMathLib.powWad(
            int256(fromBalance * W_from * FixedPointMathLib.WAD),
            oneMinusAmp
        )) / W_from;
    }

    /**
    * @notice Computes a pairwise mid price. Requires input from calcAsyncPriceFrom.
    * @param vault Vault address to examine.
    * @param toAsset Address of the token to buy.
    * @param calcAsyncPriceFromQuote The output of calcAsyncPriceFrom.
    * @return uint256 The pairwise mid price.
    */
    function calcCurrentPriceTo(
        address vault,
        address toAsset,
        uint256 calcAsyncPriceFromQuote
    ) public view returns (uint256) {
        uint256 toBalance = ERC20(toAsset).balanceOf(vault) - CatalystVaultAmplified(vault)._escrowedTokens(toAsset);
        uint256 W_to = CatalystVaultAmplified(vault)._weight(toAsset);
        int256 oneMinusAmp = getTrueAmp(vault);
        if ((calcAsyncPriceFromQuote == 0) || (W_to == 0)) return 0;

        uint256 toQuote = uint256(FixedPointMathLib.powWad(
            int256(toBalance * W_to * FixedPointMathLib.WAD),
            oneMinusAmp
        ));

        return (toQuote / calcAsyncPriceFromQuote * W_to);
    }

    /**
    * @notice Computes the current mid price. This is the current marginal price between the 2 assets.
    * @dev The mid price cannot be traded on, since the fees acts as spread.
    * @param vault Vault address to examine.
    * @param fromAsset Address of the token to sell.
    * @param toAsset Address of the token to buy.
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
