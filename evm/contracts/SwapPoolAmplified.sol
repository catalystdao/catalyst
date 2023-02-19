//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "./FixedPointMathLib.sol";
import "./CatalystIBCInterface.sol";
import "./SwapPoolCommon.sol";
import "./ICatalystV1Pool.sol";

/**
 * @title Catalyst: The Multi-Chain Swap pool
 * @author Catalyst Labs
 * @notice Catalyst multi-chain swap pool using the asset specific
 * pricing curve: 1/w^\theta (1 - \theta) where \theta is 
 * the group amplification and w is the pool balance.
 *
 * The following contract supports between 1 and 3 assets for
 * atomic swaps. To increase the number of tokens supported,
 * change MAX_ASSETS to the desired maximum token amount.
 * This constant is set in "SwapPoolCommon.sol"
 *
 * The swappool implements the ERC20 specification, such that the
 * contract will be its own pool token.
 * @dev This contract is deployed inactive: It cannot be used as a
 * swap pool as is. To use it, a proxy contract duplicating the
 * logic of this contract needs to be deployed. In Vyper, this
 * can be done through (vy >=0.3.4) create_minimal_proxy_to.
 * In Solidity, this can be done through OZ clones: Clones.clone(...)
 * After deployment of the proxy, call setup(...). This will
 * initialize the pool and prepare it for cross-chain transactions.
 *
 * If connected to a supported cross-chain interface, call
 * setConnection to connect the pool with pools on other chains.
 *
 * Finally, call finishSetup to give up the deployer's control
 * over the pool. 
 * !If finishSetup is not called, the pool can be drained!
 */
contract CatalystSwapPoolAmplified is CatalystSwapPoolCommon, ReentrancyGuard {
    using SafeTransferLib for ERC20;

    //--- ERRORS ---//
    // Errors are defined in interfaces/ICatalystV1PoolErrors.sol


    //--- Config ---//
    // Minimum time parameter adjustments can be made over.
    uint256 constant MIN_ADJUSTMENT_TIME = 7 days;
    // When the swap is a very small size of the pool, the swaps
    // returns slightly more. To counteract this, an additional fee
    // slightly larger than the error is added. The below constants
    // determines when this fee is added and the size.
    uint256 constant SMALL_SWAP_RATIO = 1e12;
    uint256 constant SMALL_SWAP_RETURN = 95e16;

    // For other config options, see SwapPoolCommon.sol

    //-- Variables --//
    int256 public _oneMinusAmp;
    int256 public _targetAmplification;

    // To keep track of group ownership, the pool needs to keep track of
    // the local unit balance. That is, do other pools own or owe assets to this pool?
    int256 public _unitTracker;

    constructor(address factory_) CatalystSwapPoolCommon(factory_) {}

    /**
     * @notice Configures an empty pool.
     * @dev If less than MAX_ASSETS are used to initiate the pool
     * let the remaining <assets> be ZERO_ADDRESS / address(0)
     *
     * Unused weights can be whatever. (0 is recommended.)
     *
     * The initial token amounts should have been sent to the pool before setup is called.
     * Since someone can call setup can claim the initial tokens, this needs to be
     * done atomically!
     *
     * If 0 of a token in assets is provided, the setup reverts.
     * @param assets A list of the token addresses associated with the pool
     * @param weights Amplified weights brings the price into a true 1:1 swap. That is:
     * i_t \cdot W_i = j_t \cdot W_j \forall i, j when P_i(i_t) = P_j(j_t).
     * in other words, weights are used to compensate for the difference in decimals. (or non 1:1 swaps.)
     * @param amp Amplification factor. Should be < 10**18.
     * @param depositor The address depositing the initial token balances.
     */
    function initializeSwapCurves(
        address[] calldata assets,
        uint256[] calldata weights,
        uint256 amp,
        address depositor
    ) public {
        require(msg.sender == FACTORY && _tokenIndexing[0] == address(0));  // dev: swap curves may only be initialized once by the factory
        // Check that the amplification is correct.
        require(amp < FixedPointMathLib.WAD);  // dev: amplification not set correctly.
        // Check for a misunderstanding regarding how many assets this pool supports.
        require(assets.length > 0 && assets.length <= MAX_ASSETS);  // dev: invalid asset count
        
        unchecked {
            _oneMinusAmp = int256(FixedPointMathLib.WAD - amp);
            _targetAmplification = int256(FixedPointMathLib.WAD - amp);
        }

        // Compute the security limit.
        uint256[] memory initialBalances = new uint256[](MAX_ASSETS);
        uint256 maxUnitCapacity = 0;
        for (uint256 it = 0; it < assets.length; ++it) {

            address tokenAddress = assets[it];
            _tokenIndexing[it] = tokenAddress;

            uint256 weight = weights[it];
            require(weight > 0);       // dev: invalid 0-valued weight provided
            _weight[tokenAddress] = weight;

            // The contract expects the tokens to have been sent to it before setup is
            // called. Make sure the pool has more than 0 tokens.
            uint256 balanceOfSelf = ERC20(tokenAddress).balanceOf(address(this));
            require(balanceOfSelf > 0); // dev: 0 tokens provided in setup.
            initialBalances[it] = balanceOfSelf;

            maxUnitCapacity += weight * balanceOfSelf;
        }

        // / The security limit is implemented as being 50% of the current balance. Since the security limit 
        // is evaluated after balance changes, the limit in storage should be the current balance.
        _maxUnitCapacity = maxUnitCapacity;

        // Mint pool tokens for pool creator.
        _mint(depositor, INITIAL_MINT_AMOUNT);

        emit Deposit(depositor, INITIAL_MINT_AMOUNT, initialBalances);
    }

    /** 
     * @notice  Returns the current cross-chain swap capacity. 
     * @dev Overwrites the common implementation because of the
     * differences as to how it is used. As a result, this always returns
     * half of the common implementation (or of _maxUnitCapacity)
     */
    function getUnitCapacity() public view override returns (uint256) {
        return super.getUnitCapacity() / 2;
    }

    /**
     * @notice Allows Governance to modify the pool weights to optimise liquidity.
     * @dev targetTime needs to be more than MIN_ADJUSTMENT_TIME in the future.
     * @param targetTime Once reached, _weight[...] = newWeights[...]
     * @param targetAmplification The new weights to apply
     */
    function modifyAmplification(
        uint256 targetTime,
        uint256 targetAmplification
    ) external onlyFactoryOwner {
        unchecked {
            require(targetTime >= block.timestamp + MIN_ADJUSTMENT_TIME); // dev: targetTime must be more than MIN_ADJUSTMENT_TIME in the future.
            require(targetTime <= block.timestamp + 365 days); // dev: Target time cannot be too far into the future.
        }
        require(targetAmplification < FixedPointMathLib.WAD);  // dev: amplification not set correctly.

        // Because of the balance0 (_unitTracker) implementation, amplification adjustment has to be disabled for cross-chain pools.
        require(_chainInterface == address(0));  // dev: Amplification adjustment is disabled for cross-chain pools.

        // Store adjustment information
        _adjustmentTarget = targetTime;
        _lastModificationTime = block.timestamp;
        unchecked {
            _targetAmplification = int256(FixedPointMathLib.WAD - targetAmplification);
        }

        emit ModifyAmplification(targetTime, targetAmplification);
    }

    /**
     * @notice If the governance requests an amplification change,
     * this function will adjust the pool weights.
     * @dev Called first thing on every function depending on amplification.
     */
    function _adjustAmplification() internal {
        // We might use adjustment target more than once. Since we don't change it, let store it.
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

                // Add the change to the current amp.
                _oneMinusAmp = currentAmplification + (
                    (targetAmplification - currentAmplification) * int256(block.timestamp - lastModification)  // timestamp is largest but small relative to int256.
                ) / int256(adjTarget - lastModification);   // adjTarget is bounded by block.timestap + 1 year
                
            }
        }
    }

    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{wA}^{wA+wx} 1/w^k · (1-k) dw
     *     = (wA + wx)^(1-k) - wA^(1-k)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current pool balance of the x token.
     * @param W The weight of the x token.
     * @param oneMinusAmp The amplification.
     * @return uint256 Group-specific units (units are **always** WAD).
     */
    function calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W,
        int256 oneMinusAmp
    ) internal pure returns (uint256) {
        return uint256(                                             // Always casts a positive value, as the first powWad term is always larger than the second one
            FixedPointMathLib.powWad(
                int256(W * (A + input) * FixedPointMathLib.WAD),    // If casting overflows to a negative number, powWad fails
                oneMinusAmp
            ) - FixedPointMathLib.powWad(
                int256(W * A * FixedPointMathLib.WAD),              // If casting overflows to a negative number, powWad fails
                oneMinusAmp
            )
        );
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
     * @param U Incoming group-specific units.
     * @param B The current pool balance of the y token.
     * @param W The weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W,
        int256 oneMinusAmp
    ) internal pure returns (uint256) {
        // W_B · B^(1-k) is repeated twice and requires 1 power.
        // As a result, we compute it and cache it.
        uint256 W_BxBtoOMA = uint256(                       // Always casts a positive value
            FixedPointMathLib.powWad(
                int256(W * B * FixedPointMathLib.WAD),      // If casting overflows to a negative number, powWad fails
                oneMinusAmp
            )
        );

        return B * (
            FixedPointMathLib.WAD - uint256(                                                        // Always casts a positive value
                FixedPointMathLib.powWad(
                    int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U, W_BxBtoOMA)),                 // Casting never overflows, as division result is always < 1
                    FixedPointMathLib.WADWAD / oneMinusAmp 
                )
            )
        ) / FixedPointMathLib.WAD;
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
     * calcPriceCurveLimit(calcPriceCurveArea(input, A, W_A, amp), B, W_B, amp).
     * @dev All input amounts should be the raw numbers and not WAD.
     * @param input The input amount.
     * @param A The current pool balance of the _in token.
     * @param B The current pool balance of the _out token.
     * @param W_A The pool weight of the _in token.
     * @param W_B The pool weight of the _out token.
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
        // )); // calcPriceCurveArea(input, A, W_A, amp)

        // return B * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
        //             int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U, W_BxBtoOMA)),
        //             int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD / uint256(oneMinusAmp)))
        //         )) / FixedPointMathLib.WAD; // calcPriceCurveLimit
        return calcPriceCurveLimit(calcPriceCurveArea(input, A, W_A, oneMinusAmp), B, W_B, oneMinusAmp);
    }


    /**
     * @notice Converts units into pool tokens with the below formula
     *      pt = PT · (((N · wa_0^(1-k) + U)/(N · wa_0^(1-k))^(1/(1-k)) - 1)
     * @dev The function leaves a lot of computation to the external implementation. This is done to avoid recomputing values several times.
     * @param U Then number of units to convert into pool tokens.
     * @param ts The current pool token supply. The escrowed pool tokens should not be added, since the function then returns more.
     * @param it_times_walpha_amped wa_0^(1-k)
     * @param oneMinusAmpInverse The pool amplification.
     * @return uint256 Output denominated in pool tokens.
     */
    function calcPriceCurveLimitShare(uint256 U, uint256 ts, uint256 it_times_walpha_amped, int256 oneMinusAmpInverse) internal pure returns (uint256) {
        uint256 poolTokens = (
            ts * uint256(  // Always casts a positive value, as powWad >= 1, hence powWad - WAD >= 0
                FixedPointMathLib.powWad(  // poWad always >= 1, as the 'base' is always >= 1
                    int256(FixedPointMathLib.divWadDown(  // If casting overflows to a negative number, powWad fails
                        it_times_walpha_amped + U,
                        it_times_walpha_amped
                    )),
                    oneMinusAmpInverse
                ) - int256(FixedPointMathLib.WAD)
            )
        ) / FixedPointMathLib.WAD;

        return poolTokens;
    }

    /**
     * @notice Computes the return of SendAsset.
     * @dev Returns 0 if from is not a token in the pool
     * @param fromAsset The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @return uint256 Group-specific units.
     */
    function calcSendAsset(
        address fromAsset,
        uint256 amount
    ) public view returns (uint256) {
        // A high => fewer units returned. Do not subtract the escrow amount
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 W = _weight[fromAsset];


        // If a token is not part of the pool, W is 0. This returns 0 since
        // 0^p = 0.
        uint256 U = calcPriceCurveArea(amount, A, W, _oneMinusAmp);

        // If the swap is a very small portion of the pool
        // Add an additional fee. This covers mathematical errors.
        if (A/SMALL_SWAP_RATIO >= amount) return U * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;
        
        return U;
    }

    /**
     * @notice Computes the output of ReceiveSwap.
     * @dev Reverts if to is not a token in the pool
     * @param toAsset The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveSwap(
        address toAsset,
        uint256 U
    ) public view returns (uint256) {
        // B low => fewer tokens returned. Subtract the escrow amount to decrease the balance.
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - _escrowedTokens[toAsset];
        uint256 W = _weight[toAsset];

        // If someone were to purchase a token which is not part of the pool on setup
        // they would just add value to the pool. We don't care about it.
        // However, it will revert since the solved integral contains U/W and when
        // W = 0 then U/W returns division by 0 error.
        return calcPriceCurveLimit(U, B, W, _oneMinusAmp);
    }

    /**
     * @notice Computes the output of localSwap.
     * @dev Implemented through calcCombinedPriceCurves.
     * @param fromAsset The address of the token to sell.
     * @param toAsset The address of the token to buy.
     * @param amount The amount of from token to sell for to token.
     * @return uint256 Output denominated in toAsset.
     */
    function calcLocalSwap(
        address fromAsset,
        address toAsset,
        uint256 amount
    ) public view returns (uint256) {
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - _escrowedTokens[toAsset];
        uint256 W_A = _weight[fromAsset];
        uint256 W_B = _weight[toAsset];
        int256 oneMinusAmp = _oneMinusAmp;

        uint256 output = calcCombinedPriceCurves(amount, A, B, W_A, W_B, oneMinusAmp);

        // If the swap is a very small portion of the pool
        // Add an additional fee. This covers mathematical errors.
        if (A/SMALL_SWAP_RATIO >= amount) return output * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;

        return output;
    }

    /**
     * @notice Deposits a  user-configurable amount of tokens.
     * @dev Requires approvals for all tokens within the pool.
     * It is advised that the deposit matches the pool's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of pool tokens to be minted.
     */
    function depositMixed(
        uint256[] calldata tokenAmounts,
        uint256 minOut
    ) nonReentrant external returns(uint256) {
        _adjustAmplification();
        int256 oneMinusAmp = _oneMinusAmp;

        uint256 walpha_0_ampped;

        // There is a Stack too deep issue in a later branch. To counteract this,
        // wab is stored short-lived. This requires letting U get negative.
        // As such, we define an additional variable called intU which is signed
        int256 U;
        uint256 it;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            int256 weightedAssetBalanceSum = 0;
            uint256 assetDepositSum = 0;
            for (it = 0; it < MAX_ASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // Not minus escrowedAmount, since we want the deposit to return less.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this));

                // Store the amount deposited to later be used for modifying the security limit.
                assetDepositSum += tokenAmounts[it] * weight;
                
                {
                    // wa^(1-k) is required twice. It is F(A) in the
                    // sendAsset equation and part of the wa_0^(1-k) calculation
                    int256 wab = FixedPointMathLib.powWad(
                        int256(weightAssetBalance * FixedPointMathLib.WAD),     // If casting overflows to a negative number, powWad fails
                        oneMinusAmp
                    );
                    weightedAssetBalanceSum += wab;

                    // This line is the origin of the stack too deep issue.
                    // since it implies we cannot move intU += before this section.
                    // which would solve the issue.
                    // Save gas if the user provides no tokens, as the rest of the loop has no effect in that case
                    if (tokenAmounts[it] == 0) continue;
                    
                    // int_A^{A+x} f(w) dw = F(A+x) - F(A).
                    // This is -F(A). Since we are subtracting first,
                    // U must be able to go negative.
                    U -= wab;
                }

                // Add F(A+x)
                U += FixedPointMathLib.powWad(
                    int256((weightAssetBalance + weight * tokenAmounts[it]) * FixedPointMathLib.WAD),   // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );
                
                ERC20(token).safeTransferFrom(
                    msg.sender,
                    address(this),
                    tokenAmounts[it]
                );  // dev: Token withdrawal from user failed.
            }
            // Increase the security limit by the amount deposited.
            _maxUnitCapacity += assetDepositSum;
            // Short term decrease the security limit by the amount deposited.
            _usedUnitCapacity += assetDepositSum;

            // Compute the reference liquidity.
            // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
            // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
            // _unitTracker < weightedAssetBalance0Sum
            walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / it;     // By design, weightedAssetBalanceSum > _unitTracker
        }

        // Subtract fee from U. This stops people from using deposit and withdrawal as a method of swapping.
        // To reduce costs, there the governance fee is not included. This does result in deposit+withdrawal
        // working as a way to circumvent the governance fee.
        // U shouldn't ever be negative. But in the case it is, the result is very bad. For safety, check it is above 0.
        require(U >= 0); // dev: U needs to be positive, otherwise, the uint256 casting becomes too larger.
        unchecked {
            // U is generally small, so the below equation should not overflow.
            // If it does, it has to be (uint256(U) * (FixedPointMathLib.WAD - _poolFee)) that overflows.
            // In which case, something close to 0 will be returned. When divided by FixedPointMathLib.WAD
            // it will return 0.
            // The casting to int256 is then 0.

            // If the overflow doesn't happen inside uint256(...), 'U' will be smaller because of the division by FixedPointMathLib.WAD. Thus the int256 casting will now overflow.
            U = int256(
                // U shouldn't be negative but the above check ensures it is ALWAYS positive.
                (uint256(U) * (FixedPointMathLib.WAD - _poolFee))/FixedPointMathLib.WAD
            );
        }

        int256 oneMinusAmpInverse = FixedPointMathLib.WADWAD / oneMinusAmp;

        uint256 it_times_walpha_amped = it * walpha_0_ampped;

        // On totalSupply. Do not add escrow amount, as higher amount
        // results in a larger return.
        uint256 poolTokens = calcPriceCurveLimitShare(uint256(U), totalSupply, it_times_walpha_amped, oneMinusAmpInverse);  // uint256: U is positive by design.

        // Check if the return satisfies the user.
        if (minOut > poolTokens)
            revert ReturnInsufficient(poolTokens, minOut);

        // Mint the desired number of pool tokens to the user.
        _mint(msg.sender, poolTokens);

        emit Deposit(msg.sender, poolTokens, tokenAmounts);

        return poolTokens;
    }

    /**
     * @notice Burns poolTokens and releases the symmetrical share of tokens to the burner. 
     * This doesn't change the pool price.
     * @dev This is the cheapest way to withdraw.
     * poolTokens == 0 or very small results: revert: Integer overflow. See note for why.
     * @param poolTokens The number of pool tokens to burn.
     * @param minOut The minimum token output. If less is returned, the transaction reverts.
     * @return uint256[] memory An array containing the amounts withdrawn.
     */
    function withdrawAll(
        uint256 poolTokens,
        uint256[] calldata minOut
    ) nonReentrant external returns(uint256[] memory) {
        _adjustAmplification();
        // Burn the desired number of pool tokens to the user.
        // If they don't have it, it saves gas.
        // * Remember to add poolTokens when accessing totalSupply
        _burn(msg.sender, poolTokens);

        int256 oneMinusAmp = _oneMinusAmp;

        // Cache weights and balances.
        address[MAX_ASSETS] memory tokenIndexed;
        uint256[MAX_ASSETS] memory weightAssetBalances;
        int256[MAX_ASSETS] memory ampWeightAssetBalances;

        uint256 walpha_0_ampped;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            int256 weightedAssetBalanceSum = 0;
            // "it" is needed briefly outside the loop.
            uint256 it;
            for (it = 0; it < MAX_ASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                tokenIndexed[it] = token;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);
                weightAssetBalances[it] = weightAssetBalance; // Store 

                int256 wab = FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),     // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );
                ampWeightAssetBalances[it] = wab; // Store
                weightedAssetBalanceSum += wab;
            }

            // Compute the reference liquidity.
            // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
            // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
            // _unitTracker < weightedAssetBalance0Sum
            walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / it;     // By design, weightedAssetBalanceSum > _unitTracker
        }


        // For later event logging, the transferred tokens are stored.
        uint256[] memory amounts = new uint256[](minOut.length < MAX_ASSETS ? minOut.length : MAX_ASSETS);
        {
            // wtk = (wa^(1-k) + (wa_0 + wpt)^(1-k) - wa_0^(1-k)))^(1/(1-k)) - wa
            // The inner diff is (wa_0 + wpt)^(1-k) - wa_0^(1-k).
            // since it doesn't depend on the token, it should only be computed once
            // The following is a reduction of the equation to reduce costs.
            uint256 innerdiff;
            { 
                // Remember to add the number of pool tokens burned to totalSupply
                // _escrowedPoolTokens is added, since it makes pt_fraction smaller
                uint256 ts = (totalSupply + _escrowedPoolTokens + poolTokens);
                uint256 pt_fraction = ((ts + poolTokens) * FixedPointMathLib.WAD) / ts;

                // The reduced equation:
                innerdiff = FixedPointMathLib.mulWadDown(
                    walpha_0_ampped, 
                    uint256(FixedPointMathLib.powWad(       // Always casts a positive value
                        int256(pt_fraction),                // If casting overflows to a negative number, powWad fails
                        oneMinusAmp
                    )) - FixedPointMathLib.WAD
                );
            }

            int256 oneMinusAmpInverse = FixedPointMathLib.WADWAD / oneMinusAmp;

            uint256 totalWithdrawn = 0;
            for (uint256 it = 0; it < MAX_ASSETS; ++it) {
                address token = tokenIndexed[it];
                if (token == address(0)) break;

                // wtk = (wa^(1-k) + (wa_0 + wpt)^(1-k) - wa_0^(1-k)))^(1/(1-k)) - wa
                // wtk = (wa^(1-k) + innerDiff)^(1/(1-k)) - wa
                // note: This underflows if innerdiff is very small / 0.
                // Since ampWeightAssetBalances ** (1/(1-amp)) == weightAssetBalances but the
                // mathematical lib returns ampWeightAssetBalances ** (1/(1-amp)) < weightAssetBalances.
                // the result is that if innerdiff isn't big enough to make up for the difference
                // the transaction reverts. This is "okay", since it means fewer tokens are returned.
                uint256 weightedTokenAmount = (uint256(FixedPointMathLib.powWad(        // Always casts a positive value
                    ampWeightAssetBalances[it] + int256(innerdiff),             // If casting overflows, either less is returned (if addition is positive) or powWad fails (if addition is negative)
                    oneMinusAmpInverse // 1/(1-amp)
                )) - (weightAssetBalances[it] * FixedPointMathLib.WAD)) / FixedPointMathLib.WAD;

                //! If the pool doesn't have enough assets for a withdrawal, then
                //! withdraw all of the pools assets. This should be protected against by setting minOut != 0.
                //! This happens because the pool expects assets to come back. (it is owed assets)
                //! We don't want to keep track of debt so we simply return less
                if (weightedTokenAmount > weightAssetBalances[it]) {
                    // Set the token amount to the pool balance.
                    // Recalled that the escrow balance is subtracted from weightAssetBalances.
                    weightedTokenAmount = weightAssetBalances[it];
                }

                // Store the amount withdrawn to subtract from the security limit later.
                totalWithdrawn +=  weightedTokenAmount;

                // remove the weight from weightedTokenAmount.
                weightedTokenAmount /= _weight[token];

                // Check if the user is satisfied with the output.
                if (minOut[it] > weightedTokenAmount)
                    revert ReturnInsufficient(weightedTokenAmount, minOut[it]);

                // Store the token amount.
                amounts[it] = weightedTokenAmount;

                // Transfer the released tokens to the user.
                ERC20(token).safeTransfer(msg.sender, weightedTokenAmount);
            }

            // Decrease the security limit by the amount withdrawn.
            _maxUnitCapacity -= totalWithdrawn;
            if (_usedUnitCapacity <= totalWithdrawn) {
                _usedUnitCapacity = 0;
            } else {
                _usedUnitCapacity -= totalWithdrawn;
            }
    
        }

        // Emit the event
        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

    /**
     * @notice Burns poolTokens and release a token distribution which can be set by the user.
     * @dev It is advised that the withdrawal matches the pool's %token distribution.
     * @param poolTokens The number of pool tokens to withdraw
     * @param withdrawRatio The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is WAD.
     * @param minOut The minimum number of tokens withdrawn.
     * @return uint256[] memory An array containing the amounts withdrawn
     */
    function withdrawMixed(
        uint256 poolTokens,
        uint256[] calldata withdrawRatio,
        uint256[] calldata minOut
    ) nonReentrant external returns(uint256[] memory) {
        _adjustAmplification();
        // Burn the desired number of pool tokens to the user.
        // If they don't have it, it saves gas.
        // * Remember to add poolTokens when accessing totalSupply
        _burn(msg.sender, poolTokens);

        int256 oneMinusAmp = _oneMinusAmp;

        // Cache weights and balances.
        address[MAX_ASSETS] memory tokenIndexed;
        uint256[MAX_ASSETS] memory assetBalances;
        int256[MAX_ASSETS] memory ampWeightAssetBalances;

        uint256 U = 0;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        // unlike in withdrawAll, this value is only needed to compute U.
        {
            // As such, we don't need to remember the value beyond this section.
            uint256 walpha_0_ampped;
            {
                int256 weightedAssetBalanceSum = 0;
                // A very careful stack optimisation is made here.
                // "it" is needed briefly outside the loop. However, to reduce the number
                // of items in the stack, U = it.
                for (U = 0; U < MAX_ASSETS; ++U) {
                    address token = _tokenIndexing[U];
                    if (token == address(0)) break;
                    tokenIndexed[U] = token;
                    uint256 weight = _weight[token];

                    // minus escrowedAmount, since we want the withdrawal to return less.
                    uint256 ab = (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);
                    assetBalances[U] = ab;
                    uint256 weightAssetBalance = weight * ab;

                    int256 wab = FixedPointMathLib.powWad(
                        int256(weightAssetBalance * FixedPointMathLib.WAD),     // If casting overflows to a negative number, powWad fails
                        oneMinusAmp
                    );
                    ampWeightAssetBalances[U] = wab; // Store since it is an expensive calculation.
                    weightedAssetBalanceSum += wab;
                }

                // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
                // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
                // _unitTracker < weightedAssetBalance0Sum
                walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / U;     // By design, weightedAssetBalanceSum > _unitTracker

                // set U = number of tokens in the pool. But that is exactly what it is.
            }
            // Remember to add the number of pool tokens burned to totalSupply
            uint256 ts = totalSupply + _escrowedPoolTokens + poolTokens;
            uint256 pt_fraction = FixedPointMathLib.divWadDown(ts + poolTokens, ts);

            // Compute the unit worth of the pool tokens.
            // Recall that U is equal to N already. So we only need to multiply by the right side.
            U *= FixedPointMathLib.mulWadDown(
                walpha_0_ampped, 
                uint256(FixedPointMathLib.powWad(       // Always casts a positive value
                    int256(pt_fraction),                // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                )) - FixedPointMathLib.WAD
            );
        }
        
        int256 oneMinusAmpInverse = FixedPointMathLib.WADWAD / oneMinusAmp;

        // For later event logging, the transferred tokens are stored.
        uint256[] memory amounts = new uint256[](minOut.length < MAX_ASSETS ? minOut.length : MAX_ASSETS);
        uint256 totalWithdrawn = 0;
        for (uint256 it = 0; it < MAX_ASSETS; ++it) {
            if (tokenIndexed[it] == address(0)) break;

            uint256 U_i = (U * withdrawRatio[it]) / FixedPointMathLib.WAD;
            if (U_i == 0) {
                if (minOut[it] != 0)
                    revert ReturnInsufficient(0, minOut[it]);
                continue;
            }
            U -= U_i;  // Subtract the number of units used. This will underflow for malicious withdrawRatios > 1.

            // uint256 tokenAmount = calcReceiveSwap(_tokenIndexing[it], U_i);
            
            // W_B · B^(1-k) is required twice and requires 1 power. We already computed it:
            uint256 W_BxBtoOMA = uint256(ampWeightAssetBalances[it]);   // Always casts a positive value
            uint256 tokenAmount = (
                assetBalances[it] * (
                    FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(               // Always casts a positive value, and powWad is always <= 1, as 'base' is <= 1
                        int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U_i, W_BxBtoOMA)),   // Casting never overflows, as division result is always <= 1
                        oneMinusAmpInverse // 1/(1-amp))
                    ))
                )
            ) / FixedPointMathLib.WAD;

            // Ensure the output satisfies the user.
            if (minOut[it] > tokenAmount)
                revert ReturnInsufficient(tokenAmount, minOut[it]);

            // Store amount for withdraw event
            amounts[it] = tokenAmount;

            // Transfer the released tokens to the user.
            ERC20(tokenIndexed[it]).safeTransfer(msg.sender, tokenAmount);

            // Decrease the security limit by the amount withdrawn.
            totalWithdrawn += tokenAmount * _weight[tokenIndexed[it]];
        }
        _maxUnitCapacity -= totalWithdrawn;
        if (_usedUnitCapacity <= totalWithdrawn) {
            _usedUnitCapacity = 0;
        } else {
            _usedUnitCapacity -= totalWithdrawn;
        }

        // Emit the event
        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

    /**
     * @notice A swap between 2 assets within the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of toAsset the user wants.
     * @return uint256 The number of tokens purchased.
     */
    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) nonReentrant external returns (uint256) {
        _adjustAmplification();
        uint256 fee = FixedPointMathLib.mulWadDown(amount, _poolFee);

        // Calculate the return value.
        uint256 out = calcLocalSwap(fromAsset, toAsset, amount - fee);

        // Check if the calculated returned value is more than the minimum output.
        if (minOut > out) revert ReturnInsufficient(out, minOut);

        // Transfer tokens to the user and collect tokens from the user.
        ERC20(toAsset).safeTransfer(msg.sender, out);
        ERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // Governance Fee
        collectGovernanceFee(fromAsset, fee);

        // For amplified pools, the security limit is based on the sum of the tokens
        // in the pool.
        if (out > amount) {
            _maxUnitCapacity -= out - amount;
        } else {
            _maxUnitCapacity += amount - out;
        }

        emit LocalSwap(msg.sender, fromAsset, toAsset, amount, out);

        return out;
    }

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another pool.
     * @dev To encode addresses in bytes32 the functions below can be used:
     * Vyper: convert(<poolAddress>, bytes32)
     * Solidity: abi.encode(<poolAddress>)
     * Brownie: brownie.convert.to_bytes(<poolAddress>, type_str="bytes32")
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in bytes32.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param amount The number of fromAsset to sell to the pool.
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address
     * @param calldata_ Data field if a call should be made on the target chain. 
     * Should be encoded abi.encode(<address>,<data>)
     * @return uint256 The number of units minted.
     */
    function sendAsset(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        bytes memory calldata_
    ) public returns (uint256) {

        // Only allow connected pools
        if (!_poolConnection[channelId][toPool]) revert PoolNotConnected(channelId, toPool);

        require(fallbackUser != address(0));
        _adjustAmplification();
        uint256 fee = FixedPointMathLib.mulWadDown(amount, _poolFee);

        // Calculate the group-specific units bought.
        uint256 U = calcSendAsset(
            fromAsset,
            amount - fee
        );

        // sendAssetAck requires casting U to int256 to update the _unitTracker and must never revert. Check for overflow here.
        require(U < uint256(type(int256).max));     // int256 max fits in uint256
        _unitTracker += int256(U);

        // Only need to hash info that is required by the escrow (+ some extra for randomisation)
        // No need to hash context (as token/liquidity escrow data is different), fromPool, toPool, targetAssetIndex, minOut, CallData
        bytes32 assetSwapHash = computeAssetSwapHash(
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            amount - fee,   // Required! to validate release escrow data
            fromAsset,      // Required! to validate release escrow data
            uint32(block.number % 2**32)
        );

        // Wrap the escrow information into a struct. This reduces the stack-print.
        AssetSwapMetadata memory swapMetadata = AssetSwapMetadata({
            fromAmount: amount - fee,
            fromAsset: fromAsset,
            swapHash: assetSwapHash,
            blockNumber: uint32(block.number % 2**32)
        });

        // Send the purchased units to toPool on the target chain.
        CatalystIBCInterface(_chainInterface).sendCrossChainAsset(
            channelId,
            toPool,
            toAccount,
            toAssetIndex,
            U,
            minOut,
            swapMetadata,
            calldata_
        );

        // Escrow the tokens used to purchase units. These will be sent back if transaction
        // doesn't arrive / timeout.
        require(_escrowedFor[assetSwapHash] == address(0)); // dev: Escrow already exists.
        _escrowedTokens[fromAsset] += amount - fee;
        _escrowedFor[assetSwapHash] = fallbackUser;

        // Governance Fee
        collectGovernanceFee(fromAsset, fee);

        // Collect the tokens from the user.
        ERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit.

        emit SendAsset(
            toPool,
            toAccount,
            fromAsset,
            toAssetIndex,
            amount,
            U,
            minOut,
            assetSwapHash
        );

        return U;
    }

    /** @notice Copy of sendAsset with no calldata_ */
    function sendAsset(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256) {
        bytes memory calldata_ = new bytes(0);
        return
            sendAsset(
                channelId,
                toPool,
                toAccount,
                fromAsset,
                toAssetIndex,
                amount,
                minOut,
                fallbackUser,
                calldata_
            );
    }

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token (toAsset)
     * @dev Can only be called by the chainInterface.
     * @param channelId The incoming connection identifier.
     * @param fromPool The source pool.
     * @param toAssetIndex Index of the asset to be purchased with Units.
     * @param toAccount The recipient.
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     * @param swapHash Used to connect 2 swaps within a group. 
     */
    function receiveSwap(
        bytes32 channelId,
        bytes32 fromPool,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash
    ) public returns (uint256) {

        // Only allow connected pools
        if (!_poolConnection[channelId][fromPool]) revert PoolNotConnected(channelId, fromPool);

        // The chainInterface is the only valid caller of this function.
        require(msg.sender == _chainInterface);
        _adjustAmplification();

        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];


        // Calculate the swap return value.
        // Fee is always taken on the sending token.
        uint256 purchasedTokens = calcReceiveSwap(toAsset, U);

        // Check if the swap is according to the swap limits
        uint256 deltaSecurityLimit = purchasedTokens * _weight[toAsset];
        _maxUnitCapacity -= deltaSecurityLimit;
        updateUnitCapacity(deltaSecurityLimit);

        // Ensure the user is satisfied by the number of tokens.
        if (minOut > purchasedTokens) revert ReturnInsufficient(purchasedTokens, minOut);

        // Track units for balance0 computation.
        _unitTracker -= int256(U);

        // Send the assets to the user.
        ERC20(toAsset).safeTransfer(toAccount, purchasedTokens);

        emit ReceiveSwap(fromPool, toAccount, toAsset, U, purchasedTokens, swapHash);

        return purchasedTokens;
    }

    function receiveSwap(
        bytes32 channelId,
        bytes32 fromPool,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256) {
        uint256 purchasedTokens = receiveSwap(
            channelId,
            fromPool,
            toAssetIndex,
            toAccount,
            U,
            minOut,
            swapHash
        );

        // Let users define custom logic which should be executed after the swap.
        // The logic is not contained within a try - except so if the logic reverts
        // the transaction will timeout and the user gets the input tokens on the sending chain.
        // If this is not desired, wrap further logic in a try - except at dataTarget.
        ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, data);

        return purchasedTokens;
    }

    //--- Liquidity swapping ---//
    // Because of the way pool tokens work in a group of pools, there
    // needs to be a way for users to easily get a distributed stake.
    // Liquidity swaps is a macro implemented on the smart contract level to:
    // 1. Withdraw tokens.
    // 2. Convert tokens to units & transfer to target pool.
    // 3. Convert units to an even mix of tokens.
    // 4. Deposit the even mix of tokens.
    // In 1 user invocation.

    /**
     * @notice Initiate a cross-chain liquidity swap by withdrawing tokens and converting them to units.
     * @dev No reentry protection since only trusted contracts are called.
     * While the description says tokens are withdrawn and then converted to units, pool tokens are converted
     * directly into units through the following equation:
     *      U = N · wa^(1-k) · (((PT + pt)/PT)^(1-k) - 1)
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in bytes32.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param poolTokens The number of pool tokens to exchange
     * @param minOut The minimum number of pool tokens to mint on target pool.
     */
    function sendLiquidity(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        uint256 poolTokens,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256) {

        // Only allow connected pools
        if (!_poolConnection[channelId][toPool]) revert PoolNotConnected(channelId, toPool);

        // Address(0) is not a valid fallback user. (As checking for escrow overlap
        // checks if the fallbackUser != address(0))
        require(fallbackUser != address(0));
        // Update amplification
        _adjustAmplification();

        _burn(msg.sender, poolTokens);

        int256 oneMinusAmp = _oneMinusAmp;

        uint256 walpha_0_ampped;
        uint256 it;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            int256 weightedAssetBalanceSum = 0;
            for (it = 0; it < MAX_ASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                // A smaller number here means fewer units are transferred.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);

                weightedAssetBalanceSum += FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),     // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );
            }

            // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
            // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
            // _unitTracker < weightedAssetBalance0Sum
            walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / it;     // By design, weightedAssetBalanceSum > _unitTracker 
        }

        uint256 U = 0;
        {
            // Plus _escrowedPoolTokens since we want the withdrawal to return less. Adding poolTokens as these have already been burnt.
            uint256 ts = totalSupply + _escrowedPoolTokens + poolTokens;
            uint256 pt_fraction = FixedPointMathLib.divWadDown(ts + poolTokens, ts);

            U = it * FixedPointMathLib.mulWadDown(
                walpha_0_ampped, 
                uint256(FixedPointMathLib.powWad(       // Always casts a positive value
                    int256(pt_fraction),                // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                )) - FixedPointMathLib.WAD
            );
            // sendLiquidityAck requires casting U to int256 to update the _unitTracker and must never revert. Check for overflow here.
            require(U < uint256(type(int256).max));     // int256 max fits in uint256
            _unitTracker += int256(U);
        }

        // Only need to hash info that is required by the escrow (+ some extra for randomisation)
        // No need to hash context (as token/liquidity escrow data is different), fromPool, toPool, targetAssetIndex, minOut, CallData
        bytes32 sendLiquidityHash = computeLiquiditySwapHash(
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            poolTokens,     // Required! to validate release escrow data
            uint32(block.number % 2**32)
        );

        // Wrap the escrow information into a struct. This reduces the stack-print.
        // (Not really since only pool tokens are wrapped.)
        // However, the struct keeps the structure of swaps similar.
        LiquiditySwapMetadata memory escrowInformation = LiquiditySwapMetadata({
            fromAmount: poolTokens,
            swapHash: sendLiquidityHash,
            blockNumber: uint32(block.number % 2**32)
        });

        // Transfer the units to the target pools.
        CatalystIBCInterface(_chainInterface).sendCrossChainLiquidity(
            channelId,
            toPool,
            toAccount,
            U,
            minOut,
            escrowInformation
        );

        // Escrow the pool tokens
        require(_escrowedLiquidityFor[sendLiquidityHash] == address(0));
        _escrowedLiquidityFor[sendLiquidityHash] = fallbackUser;
        _escrowedPoolTokens += poolTokens;

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit at a low cost.

        emit SendLiquidity(
            toPool,
            toAccount,
            poolTokens,
            U,
            sendLiquidityHash
        );

        return U;
    }

    /**
     * @notice Completes a cross-chain liquidity swap by converting units to tokens and depositing
     * @dev No reentry protection since only trusted contracts are called.
     * Called exclusively by the chainInterface.
     * While the description says units are converted to tokens and then deposited, units are converted
     * directly to pool tokens through the following equation:
     *      pt = PT · (((N · wa_0^(1-k) + U)/(N · wa_0^(1-k))^(1/(1-k)) - 1)
     * @param fromPool The source pool
     * @param toAccount The recipient of the pool tokens
     * @param U Number of units to convert into pool tokens.
     * @param minOut Minimum number of tokens to mint. Otherwise: reject.
     * @param swapHash Used to connect 2 swaps within a group. 
     * @return uint256 Number of pool tokens minted to the recipient.
     */
    function receiveLiquidity(
        bytes32 channelId,
        bytes32 fromPool,
        address toAccount,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash
    ) external returns (uint256) {

        // Only allow connected pools
        if (!_poolConnection[channelId][fromPool]) revert PoolNotConnected(channelId, fromPool);

        // The chainInterface is the only valid caller of this function.
        require(msg.sender == _chainInterface);
        _adjustAmplification();

        int256 oneMinusAmp = _oneMinusAmp;

        uint256 walpha_0_ampped;
        uint256 it;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            int256 weightedAssetBalanceSum = 0;
            for (it = 0; it < MAX_ASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // not minus escrowedAmount, since we want the withdrawal to return less.
                // A larger number here means more units have to be transferred.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this));

                weightedAssetBalanceSum += FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),     // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );
            }

            // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
            // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
            // _unitTracker < weightedAssetBalance0Sum
            walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / it;     // By design, weightedAssetBalanceSum > _unitTracker
        }

        int256 oneMinusAmpInverse = FixedPointMathLib.WADWAD / oneMinusAmp;

        uint256 it_times_walpha_amped = it * walpha_0_ampped;

        // On totalSupply. Do not add escrow amount, as higher amount
        // results in a larger return.
        uint256 poolTokens = calcPriceCurveLimitShare(U, totalSupply, it_times_walpha_amped, oneMinusAmpInverse);

        // Check if the user would accept the mint.
        if(minOut > poolTokens) revert ReturnInsufficient(poolTokens, minOut);

        // Update the unit tracker:
        _unitTracker -= int256(U);

        // Security limit
        {
            // To calculate the poolTokenEquiv, we set \alpha_t = \alpha_0.
            // This should be a close enough approximation.
            // If U > it_times_walpha_amped, then U can purchase more than 50% of the pool.
            // And the below calculation doesn't work.
            if (it_times_walpha_amped <= U) revert ExceedsSecurityLimit(U - it_times_walpha_amped);
            uint256 poolTokenEquiv = FixedPointMathLib.mulWadUp(
                uint256(FixedPointMathLib.powWad(                           // Always casts a positive value
                    int256(it_times_walpha_amped),                          // If casting overflows to a negative number, powWad fails
                    oneMinusAmpInverse
                )),
                FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(   // powWad is always <= 1, as 'base' is always <= 1
                    int256(FixedPointMathLib.divWadDown(                    // Casting never overflows, as division result is always <= 1
                        it_times_walpha_amped - U,
                        it_times_walpha_amped
                    )),
                    oneMinusAmpInverse
                ))
            );
            // Check if the swap is according to the swap limits
            updateUnitCapacity(poolTokenEquiv * 2 / FixedPointMathLib.WAD);
        }

        // Mint pool tokens for the user.
        _mint(toAccount, poolTokens);

        emit ReceiveLiquidity(fromPool, toAccount, U, poolTokens, swapHash);

        return poolTokens;
    }

    //-- Escrow Functions --//

    /** 
     * @notice Deletes and releases escrowed tokens to the pool and updates the security limit.
     * @dev Should never revert!  
     * The base implementation exists in CatalystSwapPoolCommon. The function adds security limit
     * adjustment to the implementation to swap volume supported.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function sendAssetAck(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) public override {
        // Execute common escrow logic.
        super.sendAssetAck(toAccount, U, escrowAmount, escrowToken, blockNumberMod);

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        unchecked {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UC = _usedUnitCapacity;
            // If UC < escrowAmount and we do UC - escrowAmount < 0 underflow => bad.
            if (UC > escrowAmount) {
                _usedUnitCapacity = UC - escrowAmount; // Does not underflow since _usedUnitCapacity > escrowAmount.
            } else if (UC != 0) {
                // If UC == 0, then we shouldn't do anything. Skip that case.
                // when UC <= escrowAmount => UC - escrowAmount <= 0 => max(UC - escrowAmount, 0) = 0
                _usedUnitCapacity = 0;
            }
            _maxUnitCapacity += escrowAmount * _weight[escrowToken];  // Does not overflow, since weight times balance of the pool doesn't overflow. 
        }
    }

    /** 
     * @notice Deletes and releases escrowed tokens to the pool and updates the security limit.
     * @dev Should never revert!
     * The base implementation exists in CatalystSwapPoolCommon. The function adds security limit
     * adjustment to the implementation to swap volume supported.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units acquired.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function sendAssetTimeout(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) public override {
        // Execute common escrow logic.
        super.sendAssetTimeout(toAccount, U, escrowAmount, escrowToken, blockNumberMod);

        // Removed timed-out units from the unit tracker. This will keep the
        // balance0 in balance, since tokens also leave the pool
        _unitTracker -= int256(U);      // It has already been checked on sendAsset that casting to int256 will not overflow.
                                        // Cannot be manipulated by the router as, otherwise, the swapHash check will fail
    }

    // sendLiquidityAck is not overwritten since we are unable to increase
    // the security limit. This is because it is very expensive to compute the update
    // to the security limit. If someone liquidity swapped a significant amount of assets
    // it is assumed the pool has low liquidity. In these cases, liquidity swaps shouldn't be used.

    /** 
     * @notice Implements basic liquidity ack logic: Deletes and releases pool tokens to the pool.
     * @dev Should never revert!  
     * The base implementation exists in CatalystSwapPoolCommon.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function sendLiquidityTimeout(
        bytes32 toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) public virtual override {
        super.sendLiquidityTimeout(toAccount, U, escrowAmount, blockNumberMod);

        // Removed timed-out units from the unit tracker. This will keep the
        // balance0 in balance, since tokens also leave the pool
        _unitTracker -= int256(U);      // It has already been checked on sendAsset that casting to int256 will not overflow.
                                        // Cannot be manipulated by the router as, otherwise, the swapHash check will fail
    }

}
