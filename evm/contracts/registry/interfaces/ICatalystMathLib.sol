//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import "../FixedPointMathLib.sol";
import "../interfaces/ICatalystV1PoolDerived.sol";
import "../interfaces/ICatalystV1PoolState.sol";

/**
 * @title Catalyst: The Multi-Chain Swap pool
 * @author Catalyst Labs
 * @notice This contract is not optimised and serves to aid in off-chain quering.
 */
contract CatalystMathLib {
    
    // This function serves to get the actual weight. If weights are being adjusted, the pure pool weights might lie.
    function getTrueWeight(address vault, address asset) external view returns(uint256);

    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{A}^{A+x} W/w dw = W ln((A+x)/A)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current pool balance of the x token.
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
     * @param B The current pool balance of the y token.
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
     * @param A The current pool balance of the x token.
     * @param B The current pool balance of the y token.
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
    ) external pure returns (uint256);

    /**
     * @notice Solves the generalised swap integral.
     * @dev Based on _calcPriceCurveLimit but the multiplication by the
     * specific token is never done.
     * @param U Input units.
     * @param W The generalised weights.
     * @return uint256 Output denominated in pool share.
     */
    function calcPriceCurveLimitShare(
        uint256 U,
        uint256 W
    ) external pure returns (uint256);

    /**
     * @notice Computes the return of SendAsset.
     * @dev Returns 0 if from is not a token in the pool
     * @param fromAsset The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @return uint256 Group-specific units.
     */
    function calcSendAsset(
        address vault,
        address fromAsset,
        uint256 amount
    ) external view override returns (uint256);

    /**
     * @notice Computes the output of ReceiveAsset.
     * @dev Reverts if to is not a token in the pool
     * @param toAsset The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveAsset(
        address vault,
        address toAsset,
        uint256 U
    ) external view override returns (uint256);

    /**
     * @notice Computes the output of localSwap.
     * @dev If the pool weights of the 2 tokens are equal, a very simple curve is used.
     * If from or to is not part of the pool, the swap will either return 0 or revert.
     * If both from and to are not part of the pool, the swap can actually return a positive value.
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
    ) external view override returns (uint256);

    /**
    * @notice Computes part of the mid price. Requires calcCurrentPriceTo to convert into a pairwise price.
    * @param vault The vault address to examine.
    * @param fromAsset The address of the token to sell.
    */
    function calcAsyncPriceFrom(
        address vault,
        address fromAsset
    ) external pure returns (uint256);

    function calcCurrentPriceTo(
        address vault,
        address toAsset,
        address calcAsyncPriceFromQuote
    ) external pure returns (uint256);


    function calcCurrentPrice(
        address vault,
        address fromAsset,
        address toAsset
    ) external pure returns (uint256);
}

