//SPDX-License-Identifier: MIT

pragma solidity ^0.8.17;

import {ERC20} from 'solmate/tokens/ERC20.sol';
import "../../interfaces/ICatalystV1VaultDerived.sol";
import "../../interfaces/ICatalystV1VaultState.sol";
import "./ICatalystMathLibCommon.sol";

/**
 * @title Catalyst: The Multi-Chain Swap vault
 * @author Catalyst Labs
 * @notice This contract is not optimised and serves to aid in off-chain quering.
 */
interface ICatalystMathLibAmp is ICatalystMathLib {
    // This function serves to get the actual amplification. If amp is being adjusted, the pure vault amp might lie.
    function getTrueAmp(address vault) external view returns(int256);
    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{wA}^{wA+wx} 1/w^k · (1-k) dw
     *     = (wA + wx)^(1-k) - wA^(1-k)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current vault balance of the x token.
     * @param W The weight of the x token.
     * @param oneMinusAmp The amplification.
     * @return uint256 Group-specific units (units are **always** WAD).
     */
    function calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W,
        int256 oneMinusAmp
    ) external pure returns (uint256);

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
     * @param U Incoming group-specific units.
     * @param B The current vault balance of the y token.
     * @param W The weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W,
        int256 oneMinusAmp
    ) external pure returns (uint256);

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
     * @param A The current vault balance of the _in token.
     * @param B The current vault balance of the _out token.
     * @param W_A The vault weight of the _in token.
     * @param W_B The vault weight of the _out token.
     * @param oneMinusAmp The amplification.
     * @return uint256 Output denominated in output token.
     */
    function calcCombinedPriceCurves(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B,
        int256 oneMinusAmp
    ) external pure returns (uint256);

    /**
     * @notice Converts units into vault tokens with the below formula
     *      pt = PT · (((N · wa_0^(1-k) + U)/(N · wa_0^(1-k))^(1/(1-k)) - 1)
     * @dev The function leaves a lot of computation to the external implementation. This is done to avoid recomputing values several times.
     * @param U Then number of units to convert into vault tokens.
     * @param ts The current vault token supply. The escrowed vault tokens should not be added, since the function then returns more.
     * @param it_times_walpha_amped wa_0^(1-k)
     * @param oneMinusAmpInverse The vault amplification.
     * @return uint256 Output denominated in vault tokens.
     */
    function calcPriceCurveLimitShare(
        uint256 U,
        uint256 ts, uint256 it_times_walpha_amped, int256 oneMinusAmpInverse
    ) external pure returns (uint256);
}

