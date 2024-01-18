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
interface ICatalystMathLibVol is ICatalystMathLib {
    // This function serves to get the actual weight. If weights are being adjusted, the pure vault weights might lie.
    function getTrueWeight(address vault, address asset) external view returns(uint256);

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
    ) external pure returns (uint256);

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
    ) external pure returns (uint256);

    /**
     * @notice Solves the equation
     *     \int_{A}^{A+x} W_a/w dw = \int_{B-y}^{B} W_b/w dw for y = B · (1 - ((A+x)/A)^(-W_a/W_b))
     *
     * Alternatively, the integral can be computed through:
     *      _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A), B, W_B).
     * @dev All input amounts should be the raw numbers and not WAD.
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
    ) external view returns (uint256);

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
    ) external view returns (uint256);
}

