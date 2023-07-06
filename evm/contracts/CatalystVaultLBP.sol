//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "./FixedPointMathLib.sol";
import "./CatalystIBCInterface.sol";
import "./CatalystVaultVolatile.sol";
import "./ICatalystV1Vault.sol";

/**
 * @title Catalyst: The Multi-Chain Vault
 * @author Cata Labs
 * @notice Catalyst multi-chain vault using the asset specific
 * pricing curve: W/w where W is an asset-specific weight and w
 * is the vault asset balance.
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
contract CatalystVaultLBP is CatalystVaultVolatile {
    error OnlySetupMasterDuringPreparation();

    using SafeTransferLib for ERC20;

    //--- ERRORS ---//
    // Errors are defined in interfaces/ICatalystV1VaultErrors.sol

    //--- Config ---//
    // Minimum time parameter adjustments can be made over.
    uint256 constant MIN_ADJUSTMENT_TIME = 1 days;

    // For other config options, see CatalystVaultVolatile.sol and CatalystVaultCommon.sol

    //-- Variables --//
    mapping(address => uint256) public _targetWeight;

    constructor(address factory_) CatalystVaultVolatile(factory_) {}

    /**
     * @notice Allows Governance to modify the vault weights to optimise liquidity.
     * @dev targetTime needs to be more than MIN_ADJUSTMENT_TIME in the future.
     * It is implied that if the existing weights are low <≈100, then 
     * the governance is not allowed to change vault weights. This is because
     * the update function is not made for large step sizes (which the steps would be if
     * trades are infrequent or weights are small).
     * Weights must not be set to 0. This allows someone to exploit the localSwap simplification
     * with a token not belonging to the vault. (Set weight to 0, localSwap from token not part of
     * the vault. Since 0 == 0 => use simplified swap curve. Swap goes through.)
     * @param targetTime Once reached, _weight[...] = newWeights[...]
     * @param newWeights The new weights to apply
     */
    function setWeights(uint256 targetTime, uint256[] calldata newWeights) internal {
        unchecked {
            require(targetTime >= block.timestamp + MIN_ADJUSTMENT_TIME); // dev: targetTime must be more than MIN_ADJUSTMENT_TIME in the future.
            require(targetTime <= block.timestamp + 365 days); // dev: Target time cannot be too far into the future.
        }

        // Save adjustment information
        _adjustmentTarget = targetTime;
        _lastModificationTime = block.timestamp;

        // Save the target weights
        for (uint256 it; it < MAX_ASSETS;) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            // Load new weights and current weights into memory to save gas
            uint256 newWeight = newWeights[it];
            uint256 currentWeight = _weight[token];
            require(newWeight != 0); // dev: newWeights must be greater than 0 to protect liquidity providers.
            require(newWeight <= currentWeight*10 && newWeight >= currentWeight/10); // dev: newWeights must be maximum a factor of 10 larger/smaller than the current weights to protect liquidity providers.
            _targetWeight[token] = newWeight;

            unchecked {
                it++;
            }
        }

        emit SetWeights(targetTime, newWeights);
    }

    /**
     * @notice If the governance requests a weight change, this function will adjust the vault weights.
     * @dev Called first thing on every function depending on weights.
     * The partial weight change algorithm is not made for large step increases. As a result, 
     * it is important that the original weights are large to increase the mathematical resolution.
     */
    function _updateWeights() internal {
        // We might use adjustment target more than once. Since we don't change it, store it.
        uint256 adjTarget = _adjustmentTarget;

        if (adjTarget != 0) {
            // We need to use lastModification multiple times. Store it.
            uint256 lastModification = _lastModificationTime;

            // If no time has passed since the last update, then we don't need to update anything.
            if (block.timestamp == lastModification) return;

            // Since we are storing lastModification, update the variable now. This avoid repetitions.
            _lastModificationTime = block.timestamp;

            uint256 wsum = 0;
            // If the current time is past the adjustment, the weights need to be finalized.
            if (block.timestamp >= adjTarget) {
                for (uint256 it; it < MAX_ASSETS;) {
                    address token = _tokenIndexing[it];
                    if (token == address(0)) break;

                    uint256 targetWeight = _targetWeight[token];

                    // Add new weight to the weight sum.
                    wsum += targetWeight;

                    // Save the new weight.
                    _weight[token] = targetWeight;

                    unchecked {
                        it++;
                    }
                }
                // Save weight sum.
                _maxUnitCapacity = wsum * FixedPointMathLib.LN2;

                // Set adjustmentTime to 0. This ensures the if statement is never entered.
                _adjustmentTarget = 0;

                return;
            }

            // Calculate partial weight change
            for (uint256 it; it < MAX_ASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;

                uint256 targetWeight = _targetWeight[token];
                uint256 currentWeight = _weight[token];
                // If the weight has already been reached, skip the mathematics.
                if (currentWeight == targetWeight) {
                    wsum += targetWeight;
                    continue;
                }

                if (targetWeight > currentWeight) {
                    // if the weights are increased then targetWeight - currentWeight > 0.
                    // Add the change to the current weight.
                    uint256 newWeight = currentWeight + (
                        (targetWeight - currentWeight) * (block.timestamp - lastModification)
                    ) / (adjTarget - lastModification);
                    _weight[token] = newWeight;
                    wsum += newWeight;
                } else {
                    // if the weights are decreased then targetWeight - currentWeight < 0.
                    // Subtract the change from the current weights.
                    uint256 newWeight = currentWeight - (
                        (currentWeight - targetWeight) * (block.timestamp - lastModification)
                    ) / (adjTarget - lastModification);
                    _weight[token] = newWeight;
                    wsum += newWeight;
                }
            }
            // Update security limit
            _maxUnitCapacity = wsum * FixedPointMathLib.LN2;
        }
    }

    function _preSwapHook() internal override {
        // During the setup phase, only allow the setup master to interact with the pool.
        if (_setupMaster != address(0)) {
            if(msg.sender == _setupMaster) revert OnlySetupMasterDuringPreparation();
        }
        _updateWeights();
    }

    function finishSetupAndStartLBP(uint256 targetTime, uint256[] calldata newWeights) external {
        finishSetup();
        setWeights(targetTime, newWeights);
    }
}
