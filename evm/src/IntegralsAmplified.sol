//SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.19;

import { FixedPointMathLib } from "solady/utils/FixedPointMathLib.sol";

import { WADWAD } from "./utils/MathConstants.sol";

/**
 * @title Catalyst: Amplified Integrals
 * @author Catalyst Labs Inc.
 */
contract IntegralsAmplified {
    /**
     * @notice Computes the integral \int_{wA}^{wA+wx} 1/w^k · (1-k) dw
     *     = (wA + wx)^(1-k) - wA^(1-k)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input Input amount.
     * @param A Current vault balance of the x token.
     * @param W Weight of the x token.
     * @param oneMinusAmp Amplification. Provided as (1-k).
     * @return uint256 Units (units are **always** WAD).
     */
    function _calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W,
        int256 oneMinusAmp
    ) internal pure returns (uint256) {
        // Will revert if W = 0. 
        // Or if A + input == 0.
        int256 calc = FixedPointMathLib.powWad(
            int256(W * (A + input) * FixedPointMathLib.WAD), // If casting overflows to a negative number, powWad fails.
            oneMinusAmp
        );

        // If the vault contains 0 assets, the below computation will fail. This is bad.
        // Instead, check if A is 0. If it is then skip because: (W · A)^(1-k) = (W · 0)^(1-k) = 0
        if (A != 0) {
            unchecked {
                // W * A * FixedPointMathLib.WAD < W * (A + input) * FixedPointMathLib.WAD 
                calc -= FixedPointMathLib.powWad(
                    int256(W * A * FixedPointMathLib.WAD), // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );
            }
        }
        
        return uint256(calc); // Casting always safe, as calc always > =0
    }

    /**
     * @notice Solves the equation U = \int_{wA-_wy}^{wA} W/w^k · (1-k) dw for y
     *     = B · (1 - (
     *             (wB^(1-k) - U) / (wB^(1-k))
     *         )^(1/(1-k))
     *     )
     * The value is returned as output token. (not WAD)
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always multiplied by WAD, the function should be treated as mathematically *native*.
     * @param U Incoming vault specific units.
     * @param B Current vault balance of the y token.
     * @param W Weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function _calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W,
        int256 oneMinusAmp
    ) internal pure returns (uint256) {
        // W_B · B^(1-k) is repeated twice and requires 1 power. As a result, we compute it and cache it.
        uint256 W_BxBtoOMA = uint256( // Always casts a positive value.
            FixedPointMathLib.powWad(
                int256(W * B * FixedPointMathLib.WAD), // If casting overflows to a negative number, powWad fails.
                oneMinusAmp
            )
        );

        return FixedPointMathLib.mulWad(
            B,
            FixedPointMathLib.WAD - uint256( // Always casts a positive value
                FixedPointMathLib.powWad(
                    int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U, W_BxBtoOMA)), // Casting never overflows, as division result is always < 1
                    WADWAD / oneMinusAmp 
                )
            )
        );
    }

    /**
     * @notice Solves the equation
     *     \int_{wA}^{wA + wx} 1/w^k · (1-k) dw = \int_{wB-wy}^{wB} 1/w^k · (1-k) dw for y
     *         => out = B · (1 - (
     *                 (wB^(1-k) - (wA+wx)^(1-k) - wA^(1-k)) / (wB^(1-k))
     *             )^(1/(1-k))
     *         )
     * Alternatively, the integral can be computed through:
     * _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A, amp), B, W_B, amp).
     * @dev All input amounts should be the raw numbers and not WAD.
     * @param input Input amount.
     * @param A Current vault balance of the x token.
     * @param B Current vault balance of the y token.
     * @param W_A Weight of the x token.
     * @param W_B Weight of the y token.
     * @param oneMinusAmp Amplification.
     * @return uint256 Output denominated in output token.
     */
    function _calcCombinedPriceCurves(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B,
        int256 oneMinusAmp
    ) internal pure returns (uint256) {
        // uint256 W_BxBtoOMA = uint256(FixedPointMathLib.powWad(
        //     int256(W_B * B * FixedPointMathLib.WAD),
        //     oneMinusAmp
        // ));

        // uint256 U = uint256(FixedPointMathLib.powWad(
        //     int256(W_A * (A + input) * FixedPointMathLib.WAD),
        //     oneMinusAmp
        // ) - FixedPointMathLib.powWad(
        //     int256(W_A * A * FixedPointMathLib.WAD),
        //     oneMinusAmp
        // )); // _calcPriceCurveArea(input, A, W_A, amp)

        // return B * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
        //             int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U, W_BxBtoOMA)),
        //             int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD / uint256(oneMinusAmp)))
        //         )) / FixedPointMathLib.WAD; // _calcPriceCurveLimit
        return _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A, oneMinusAmp), B, W_B, oneMinusAmp);
    }

    /**
     * @notice Converts units into vault tokens with the below formula
     *      pt = PT · (((N · wa_0^(1-k) + U)/(N · wa_0^(1-k))^(1/(1-k)) - 1)
     * @dev The function leaves a lot of computation to the external implementation. This is done to avoid recomputing values several times.
     * @param U Number of units to convert into vault tokens.
     * @param ts Current vault token supply. The escrowed vault tokens should not be added, since the function then returns more.
     * @param it_times_walpha_amped wa_0^(1-k)
     * @param oneMinusAmpInverse Vault amplification.
     * @return uint256 Output denominated in vault tokens.
     */
    function _calcPriceCurveLimitShare(uint256 U, uint256 ts, uint256 it_times_walpha_amped, int256 oneMinusAmpInverse) internal pure returns (uint256) {
        uint256 vaultTokens = FixedPointMathLib.mulWad(
            ts,
            uint256( // Always casts a positive value, as powWad >= 1, hence powWad - WAD >= 0
                FixedPointMathLib.powWad( // poWad always >= 1, as the 'base' is always >= 1
                    int256(FixedPointMathLib.divWad( // If casting overflows to a negative number, powWad fails
                        it_times_walpha_amped + U,
                        it_times_walpha_amped
                    )),
                    oneMinusAmpInverse
                ) - int256(FixedPointMathLib.WAD)
            )
        );

        return vaultTokens;
    }
}
