//SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

import {ERC20} from 'solady/tokens/ERC20.sol';
import "../../interfaces/ICatalystV1VaultDerived.sol";
import "../../interfaces/ICatalystV1VaultState.sol";

/**
 * @title Catalyst: The Multi-Chain Swap vault
 * @author Catalyst Labs
 * @notice This contract is not optimised and serves to aid in off-chain quering.
 */
interface ICatalystMathLib {
    //--- Swap integrals ---//

    /**
     * @notice Computes the return of SendAsset.
     * @dev Returns 0 if from is not a token in the vault
     * @param fromAsset The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @return uint256 Group-specific units.
     */
    function calcSendAsset(
        address vault,
        address fromAsset,
        uint256 amount
    ) external view returns (uint256);

    /**
     * @notice Computes the output of ReceiveAsset.
     * @dev Reverts if to is not a token in the vault
     * @param toAsset The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveAsset(
        address vault,
        address toAsset,
        uint256 U
    ) external view returns (uint256);

    /**
     * @notice Computes the output of localSwap.
     * @dev If the vault weights of the 2 tokens are equal, a very simple curve is used.
     * If from or to is not part of the vault, the swap will either return 0 or revert.
     * If both from and to are not part of the vault, the swap can actually return a positive value.
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
    ) external view returns (uint256);

    /**
    * @notice Computes part of the mid price. Requires calcCurrentPriceTo to convert into a pairwise price.
    * @param vault The vault address to examine.
    * @param fromAsset The address of the token to sell.
    */
    function calcAsyncPriceFrom(
        address vault,
        address fromAsset
    ) external view returns (uint256);

    function calcCurrentPriceTo(
        address vault,
        address toAsset,
        uint256 calcAsyncPriceFromQuote
    ) external view returns (uint256);


    function calcCurrentPrice(
        address vault,
        address fromAsset,
        address toAsset
    ) external view returns (uint256);
}

