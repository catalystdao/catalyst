//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.17;

import { FixedPointMathLib } from "./utils/FixedPointMathLib.sol";

/**
 * @title Catalyst: Volatile Integrals
 * @author Catalyst Labs
 */
contract IntegralsVolatile {
    /**
     * @notice Computes the integral \int_{A}^{A+x} W/w dw = W ln((A+x)/A)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current vault balance of the x token.
     * @param W The weight of the x token.
     * @return uint256 Units (units are **always** WAD).
     */
    function _calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W
    ) internal pure returns (uint256) {
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
     * @param U Incoming vault specific units.
     * @param B The current vault balance of the y token.
     * @param W The weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function _calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W
    ) internal pure returns (uint256) {
        return FixedPointMathLib.mulWadDown(
            B,
            FixedPointMathLib.WAD - uint256(FixedPointMathLib.expWad(-int256(U / W)))   // int256 casting is initially not safe. If overflow, the equation becomes: 1 - exp(U/W) => exp(U/W) > 1. In this case, Solidity's built-in safe math protection catches the overflow.
        );
    }

    /**
     * @notice Solves the equation
     *     \int_{A}^{A+x} W_a/w dw = \int_{B-y}^{B} W_b/w dw for y = B · (1 - ((A+x)/A)^(-W_a/W_b))
     *
     * Alternatively, the integral can be computed through:
     * _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A), B, W_B).
     * @dev All input amounts should be the raw numbers and not WAD.
     * @param input The input amount.
     * @param A The current vault balance of the x token.
     * @param B The current vault balance of the y token.
     * @param W_A The weight of the x token.
     * @param W_B The weight of the y token.
     * @return uint256 Output denominated in output token.
     */
    function _calcCombinedPriceCurves(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B
    ) internal pure returns (uint256) {
        return _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A), B, W_B);
    }

    /**
     * @notice Solves the generalised swap integral.
     * @dev Based on _calcPriceCurveLimit but the multiplication by the
     * specific token is never done.
     * @param U Input units.
     * @param W The generalised weights.
     * @return uint256 Output denominated in vault share.
     */
    function _calcPriceCurveLimitShare(
        uint256 U,
        uint256 W
    ) internal pure returns (uint256) {
        // Compute the non vault ownership share. (1 - vault ownership share)
        uint256 npos = uint256(FixedPointMathLib.expWad(-int256(U / W)));   // int256 casting is initially not safe. If overflow, the equation becomes: exp(U/W). In this case, when subtracted from 1 (later), Solidity's built-in safe math protection catches the overflow since exp(U/W) > 1.
        
        // Compute the vault owner share before liquidity has been added.
        // (solve share = pt/(PT+pt) for pt.)
        return FixedPointMathLib.divWadDown(FixedPointMathLib.WAD - npos, npos);
    }
}
