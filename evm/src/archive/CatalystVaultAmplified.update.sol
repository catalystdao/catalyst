//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.19;

import { CatalystVaultAmplified } from "../CatalystVaultAmplified.sol";
import { FixedPointMathLib } from "solady/utils/FixedPointMathLib.sol";

/**
 * @title Catalyst: The Multi-Chain Vault
 * @author Cata Labs
 * @notice Catalyst multi-chain vault using the asset specific
 * pricing curve: 1/w^\theta (1 - \theta) where \theta is 
 * the vault amplification and w is the vault asset balance.
 *
 * The following contract supports between 1 and 3 assets for
 * atomic swaps. To increase the number of tokens supported,
 * change MAX_ASSETS to the desired maximum token amount.
 * This constant is set in "CatalystVaultCommon.sol"
 *
 * This vault implements the ERC20 specification, such that the
 * contract will be its own vault token.
 * @dev This contract is deployed inactive: It cannot be used as a
 * vault as is. To use it, a proxy contract duplicating the
 * logic of this contract needs to be deployed. In Vyper, this
 * can be done through (vy >= 0.3.4) create_minimal_proxy_to.
 * In Solidity, this can be done through OZ clones: Clones.clone(...)
 * After deployment of the proxy, call setup(...) AND initializeSwapCurves(...).
 * This will initialize the vault and prepare it for cross-chain transactions.
 * However, only the Catalyst factory is allowed to perform these functions.
 *
 * If connected to a supported cross-chain interface, call
 * setConnection to connect the vault with vaults on other chains.
 *
 * Finally, call finishSetup to give up the creators's control
 * over the vault. 
 * !If finishSetup is not called, the vault can be drained by the creators!
 */
contract CatalystVaultAmplifiedUpdate is CatalystVaultAmplified {
    // ! The vault doesn't correctly use the amplification changes. To do that, _updateAmplification should be added to all functions that
    // ! reads _oneMinusAmp.

    constructor(address factory_, address mathlib_) CatalystVaultAmplified(factory_, mathlib_) {}


    /**
     * @notice Allows Governance to modify the vault weights to optimise liquidity.
     * @dev targetTime needs to be more than MIN_ADJUSTMENT_TIME in the future.
     * @param targetTime Once reached, _weight[...] = newWeights[...]
     * @param targetAmplification The new weights to apply
     */
    function setAmplification(uint256 targetTime, uint256 targetAmplification) external onlyFactoryOwner {
        unchecked {
            require(targetTime >= block.timestamp + MIN_ADJUSTMENT_TIME); // dev: targetTime must be more than MIN_ADJUSTMENT_TIME in the future.
            require(targetTime <= block.timestamp + 365 days); // dev: Target time cannot be too far into the future.
        
            uint256 currentAmplification = FixedPointMathLib.WAD - uint256(_oneMinusAmp);
            require(targetAmplification < FixedPointMathLib.WAD);  // dev: amplification not set correctly.
            // Limit the maximum allowed relative amplification change to a factor of 2. Note that this effectively 'locks'
            // the amplification if it gets intialized to 0. Similarly, the amplification will never be allowed to be set to
            // 0 if it is initialized to any other value (note how 'targetAmplification*2 >= currentAmplification' is used
            // instead of 'targetAmplification >= currentAmplification/2').

            require(targetAmplification <= currentAmplification << 2 && targetAmplification << 2 >= currentAmplification); // dev: targetAmplification must be maximum a factor of 2 larger/smaller than the current amplification to protect liquidity providers.
        
            // Because of the balance0 (_unitTracker) implementation, amplification adjustment has to be disabled for cross-chain vaults.
            require(_chainInterface == address(0));  // dev: Amplification adjustment is disabled for cross-chain vaults.

            // Save adjustment information
            _adjustmentTarget = targetTime;
            _lastModificationTime = block.timestamp;

            _targetAmplification = int256(FixedPointMathLib.WAD - targetAmplification);
        }

        emit SetAmplification(targetTime, targetAmplification);
    }

    /**
     * @notice If the governance requests an amplification change, this function will adjust the vault amplificaiton.
     * @dev Called first thing on every function depending on amplification.
     */
    function _updateAmplification() internal {
        // We might use adjustment target more than once. Since we don't change it, store it.
        uint256 adjTarget = _adjustmentTarget;

        if (adjTarget != 0) {
            // We need to use lastModification multiple times. Store it.
            uint256 lastModification = _lastModificationTime;

            // If no time has passed since the last update, then we don't need to update anything.
            if (block.timestamp == lastModification) return;

            // Since we are storing lastModification, update the variable now. This avoid repetitions.
            _lastModificationTime = block.timestamp;

            // If the current time is past the adjustment, the amplification needs to be finalized.
            if (block.timestamp >= adjTarget) {
                _oneMinusAmp = _targetAmplification;

                // Set adjustmentTime to 0. This ensures the if statement is never entered.
                _adjustmentTarget = 0;

                return;
            }

            // Calculate partial amp change
            int256 targetAmplification = _targetAmplification;  // uint256 0 < _targetAmplification < WAD
            int256 currentAmplification = _oneMinusAmp;  // uint256 0 < _oneMinusAmp < WAD

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
                // Sorry for having you go through all that to make the calculation unchecked. We need the gas savings.

                // Add the change to the current amp.
                _oneMinusAmp = currentAmplification + (
                    (targetAmplification - currentAmplification) * int256(block.timestamp - lastModification)  // timestamp is largest but small relative to int256.
                ) / int256(adjTarget - lastModification);   // adjTarget is bounded by block.timestap + 1 year
            }
            
        }
    }

}
