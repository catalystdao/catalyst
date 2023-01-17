//SPDX-License-Identifier: Unlicsened

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
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
 * change NUMASSETS to the desired maximum token amount.
 * This constant is set in "SwapPoolCommon.sol"
 *
 * The swappool implements the ERC20 specification, such that the
 * contract will be its own pool token.
 * @dev This contract is deployed inactive: It cannot be used as a
 * swap pool as is. To use it, a proxy contract duplicating the
 * logic of this contract needs to be deployed. In vyper, this
 * can be done through (vy >=0.3.4) create_minimal_proxy_to.
 * In Solidity, this can be done through OZ cllones: Clones.clone(...)
 * After deployment of the proxy, call setup(...). This will
 * initialize the pool and prepare it for cross-chain transactions.
 *
 * If connected to a supported cross-chain interface, call
 * createConnection to connect the pool with pools on other chains.
 *
 * Finally, call finishSetup to give up the deployer's control
 * over the pool. 
 * !If finishSetup is not called, the pool can be drained!
 */
contract CatalystSwapPoolAmplified is CatalystSwapPoolCommon, ReentrancyGuard {
    using SafeERC20 for IERC20;

    //--- ERRORS ---//
    // Errors are defined in interfaces/ICatalystV1PoolErrors.sol

    //--- Config ---//
    // Minimum time parameter adjustments can be made with.
    uint256 constant MIN_ADJUSTMENT_TIME = 60 * 60 * 24 * 7;

    // For other config options, see SwapPoolCommon.sol

    //-- Variables --//
    uint256 public _amp;
    uint256 public _targetAmplification;

    // In the equation for the security limit, this constant appears.
    // The constant depends only amp but not on the pool balance.
    // As a result, it is computed on pool deployment.
    uint256 _ampUnitCONSTANT;

    // To keep track of group ownership, the pool needs to keep track of
    // the local unit balance. That is, does other pools own or owe this pool assets?
    int128 public _unitTracker;

    /**
     * @notice Configures an empty pool.
     * @dev If less than NUMASSETS are used to setup the pool
     * let the remaining <init_assets> be ZERO_ADDRESS / address(0)
     *
     * Unused weights can be whatever. (0 is recommended.)
     *
     * The initial token amounts should have been sent to the pool before setup is called.
     * Since someone can call setup can claim the initial tokens, this needs to be
     * done atomically!
     *
     * If 0 of a token in init_assets is provided, the setup reverts.
     * @param init_assets A list of the token addresses associated with the pool
     * @param weights The weights associated with the tokens. 
     * If set to values with low resolotion (<= 10*5), this should be viewed as
     * opt out of governance weight adjustment. This is not enforced.
     * @param amp Amplification factor. Should be <= 10**18.
     * @param governanceFee The Catalyst governance fee portion. Is WAD.
     * @param name_ pool token name.
     * @param symbol_ pool token symbol.
     * @param chaininterface The message wrapper used by the pool.
     * Set to ZERO_ADDRESS / address(0) to opt out of cross-chain swapping.
     * @param setupMaster The address responsible for setting up the pool.
     */
    function setup(
        address[] calldata init_assets,
        uint256[] calldata weights,
        uint256 amp,
        uint256 governanceFee,
        string calldata name_,
        string calldata symbol_,
        address chaininterface,
        address setupMaster
    ) public {
        // Check that the amplification is correct.
        require(amp <= FixedPointMathLib.WAD);
        // Check for a misunderstanding regarding how many assets this pool supports.
        require(init_assets.length <= NUMASSETS);
        _amp = amp;
        _targetAmplification = amp;

        // Store the governance fee.
        _governanceFee = governanceFee;

        // Compute the security limit.
        { //  Stack limitations.
            uint256[] memory initialBalances = new uint256[](NUMASSETS);
            uint256 max_unit_inflow = 0;
            for (uint256 it = 0; it < init_assets.length; ++it) {
                address tokenAddress = init_assets[it];
                _tokenIndexing[it] = tokenAddress;
                _weight[tokenAddress] = weights[it];
                // The contract expect the tokens to have been sent to it before setup is
                // called. Make sure the pool has more than 0 tokens.

                uint256 balanceOfSelf = IERC20(tokenAddress).balanceOf(address(this));
                initialBalances[it] = balanceOfSelf;
                require(balanceOfSelf > 0); // dev: 0 tokens provided in setup.

                // The maximum unit flow is (1-2^{-(1-\theta)}) \sum W \alpha^{1-\theta}
                max_unit_inflow += weights[it] * uint256(FixedPointMathLib.powWad(int256(balanceOfSelf * FixedPointMathLib.WAD), int256(FixedPointMathLib.WAD - amp)));
            }
            
            emit Deposit(setupMaster, MINTAMOUNT, initialBalances);

            _ampUnitCONSTANT = FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
                int256(2 * FixedPointMathLib.WAD),
                -int256(FixedPointMathLib.WAD - amp)
            ));
            _max_unit_inflow = FixedPointMathLib.mulWadUp(_ampUnitCONSTANT, max_unit_inflow);
        }

        // Do common pool setup logic.
        setupBase(name_, symbol_, chaininterface, setupMaster);
    }

    // TODO: Fix security limit
    /**
     * @notice Allows Governance to modify the pool weights to optimise liquidity.
     * @dev targetTime needs to be more than MIN_ADJUSTMENT_TIME in the future.
     * !Can be abused by governance to disable the security limit!
     * @param targetTime Once reached, _weight[...] = newWeights[...]
     * @param targetAmplification The new weights to apply
     */
    function modifyAmplification(
        uint256 targetTime,
        uint256 targetAmplification
    ) external onlyFactoryOwner {
        require(targetTime >= block.timestamp + MIN_ADJUSTMENT_TIME); // dev: targetTime must be more than MIN_ADJUSTMENT_TIME in the future.

        // Store adjustment information
        _adjustmentTarget = targetTime;
        _lastModificationTime = block.timestamp;
        _targetAmplification = targetAmplification;

        // Recompute security limit.
        uint256 amp = targetAmplification;
        uint256 new_max_unit_inflow = 0;
        for (uint256 it = 0; it < NUMASSETS; ++it) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;
            uint256 balanceOfSelf = IERC20(token).balanceOf(address(this));
            new_max_unit_inflow += _weight[token] * uint256(FixedPointMathLib.powWad(
                int256(balanceOfSelf * FixedPointMathLib.WAD),
                int256(FixedPointMathLib.WAD - amp)
            ));
        }
        uint256 newAmpUnitCONSTANT = FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
            int256(2 * FixedPointMathLib.WAD),
            -int256(FixedPointMathLib.WAD - amp)
        ));
        _ampUnitCONSTANT = newAmpUnitCONSTANT;
        new_max_unit_inflow = FixedPointMathLib.mulWadDown(newAmpUnitCONSTANT, new_max_unit_inflow);

        // We don't want to spend gas to update the security limit on each swap. 
        // As a result, we "disable" it by immediately increasing it.
        // This is not an accurate way to do this but we will rely on deposits
        // and withdrawals to fix issues which are caused by the lazy security limit
        if (new_max_unit_inflow >= _max_unit_inflow) {
            // immediately set higher security limit to ensure the limit isn't reached.
            _max_unit_inflow = new_max_unit_inflow;
        }

        emit ModifyAmplification(targetTime, targetAmplification);
    }

    /**
     * @notice If the governance requests an amplification change,
     * this function will adjust the pool weights.
     * @dev Called first thing on every function depending on amplification.
     */
    function _A() internal {
        // We might use adjustment target more than once. Since we don't change it, lets store it.
        uint256 adjTarget = _adjustmentTarget;

        if (adjTarget != 0) {
            // We need to use lastModification again. Store it.
            uint256 lastModification = _lastModificationTime;
            _lastModificationTime = block.timestamp;

            // If no time has passed since last update, then we don't need to update anything.
            if (block.timestamp == lastModification) return;

            // If the current time is past the adjustment the adjustment needs to be finalized.
            if (block.timestamp >= adjTarget) {
                _amp = _targetAmplification;

                // Set adjustmentTime to 0. This ensures the if statement is never entered.
                _adjustmentTarget = 0;

                // Let deposits and withdrawals finalize the security limit change
                return;
            }
            
            // Calculate partial amp change
            uint256 targetAmplification = _targetAmplification;
            uint256 currentAmplification = _amp;

            if (targetAmplification > currentAmplification) {
                // if ta > ca, amp is increased and ta - ca > 0.
                // Add the change to the current amp.
                _amp = currentAmplification + (
                    (targetAmplification - currentAmplification) * (block.timestamp - lastModification)
                ) / (adjTarget - lastModification);
            } else {
                // if ca >= ta, amp is decreased and ta - ca < 0.
                // Subtract the change to the current amp.
                _amp = currentAmplification - (
                    (currentAmplification - targetAmplification) * (block.timestamp - lastModification)
                ) / (adjTarget - lastModification);
            }
        }
    }

    /**
     * @notice The maximum unit flow changes depending on the current
     * balance for amplified pools. The change is
     *     (1-2^(k-1)) · wb^(1-k) - (1-2^(k-1)) · wa^(1-k)
     * where b is the balance after the swap and a is the balance
     * before the swap.
     * @dev Always returns uint256. If oldBalance > newBalance,
     * the return should be negative. (but the absolute)
     * value is returned instead.
     * @param oldBalance The old balance.
     * @param newBalance The new balance.
     * @param asset The asset which is being updated.
     * @return uint256 Returns the absolute change in unit capacity Returns without the ampConstant.
     */
    function getModifyUnitCapacity(
        uint256 oldBalance,
        uint256 newBalance,
        address asset
    ) internal view returns (uint256) {
        // If the balances doesn't change, the change is 0.
        if (oldBalance == newBalance) return 0;

        // Cache the relevant amplification
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        uint256 weight = _weight[asset];
        // Notice, a > b => a^(1-k) > b^(1-k) =>
        // a <= b => a^(1-k) <= b^(1-k)
        if (oldBalance < newBalance)
            // Since a^(1-k) > b^(1-k), the return is positive
            return uint256(FixedPointMathLib.powWad(
                int256(weight * newBalance * FixedPointMathLib.WAD),
                oneMinusAmp
            ) - FixedPointMathLib.powWad(
                int256(weight * oldBalance * FixedPointMathLib.WAD),
                oneMinusAmp
            ));

        // Since a^(1-k) > b^(1-k), the return is negative
        // Since the function returns uint256, return
        // b^(1-k) - a^(1-k) instead. (since that is positive)
        return uint256(FixedPointMathLib.powWad(
            int256(weight * oldBalance * FixedPointMathLib.WAD),
            oneMinusAmp
        ) - FixedPointMathLib.powWad(
            int256(weight * newBalance * FixedPointMathLib.WAD),
            oneMinusAmp
        ));
    }

    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{wA}^{wA+wx} 1/w^k · (1-k) dw
     *     = (wA + wx)^(1-k) - wA^(1-k)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always denominated in WAD, the function
     * should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current pool balance of the x token.
     * @param W The weight of the x token.
     * @param amp The amplification.
     * @return uint256 Group specific units (units are **always** WAD).
     */
    function compute_integral(
        uint256 input,
        uint256 A,
        uint256 W,
        uint256 amp
    ) internal pure returns (uint256) {
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - amp);

        return uint256(FixedPointMathLib.powWad(
            int256(W * (A + input) * FixedPointMathLib.WAD),
            oneMinusAmp
        ) - FixedPointMathLib.powWad(
            int256(W * A * FixedPointMathLib.WAD),
            oneMinusAmp
        ));
    }

    /**
     * @notice Solves the equation U = \int_{wA-_wy}^{wA} W/w^k · (1-k) dw for y
     *     = B · (1 - (
     *             (wB^(1-k) - U) / (wB^(1-k))
     *         )^(1/(1-k))
     *     )
     * The value is returned as output token. (not WAD)
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always multiplifed by WAD, the function
     * should be treated as mathematically *native*.
     * @param U Incoming group specific units.
     * @param B The current pool balance of the y token.
     * @param W The weight of the y token.
     * @return uint25 Output denominated in output token. (not WAD)
     */
    function solve_integral(
        uint256 U,
        uint256 B,
        uint256 W,
        uint256 amp
    ) internal pure returns (uint256) {
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - amp);

        // W_B · B^(1-k) is repeated twice and requires 1 power.
        // As a result, we compute it and cache.
        uint256 W_BxBtoOMA = uint256(FixedPointMathLib.powWad(int256(W * B * FixedPointMathLib.WAD), oneMinusAmp));

        return B * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
                    int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U, W_BxBtoOMA)),
                    int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD / uint256(oneMinusAmp)))
                )) / FixedPointMathLib.WAD;
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
     * _solve_integral(_compute_integral(input, A, W_A, amp), B, W_B, amp).
     * However, _complete_integral is very slightly cheaper since it doesn't
     * compute oneMinusAmp twice. :)
     * (Apart from that, the mathematical operations are the same.)
     * @dev All input amounts should be the raw numbers and not X64.
     * @param input The input amount.
     * @param A The current pool balance of the _in token.
     * @param B The current pool balance of the _out token.
     * @param W_A The pool weight of the _in token.
     * @param W_B The pool weight of the _out token.
     * @param amp The amplification.
     * @return uint256 Output denominated in output token.
     */
    function complete_integral(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B,
        uint256 amp
    ) internal pure returns (uint256) {
        // int256 oneMinusAmp = int256(FixedPointMathLib.WAD - amp);
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
        // )); // compute_integral(input, A, W_A, amp)

        // return B * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
        //             int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U, W_BxBtoOMA)),
        //             int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD / uint256(oneMinusAmp)))
        //         )) / FixedPointMathLib.WAD; // solve_integral
        return solve_integral(compute_integral(input, A, W_A, amp), B, W_B, amp);
    }

    /**
     * @notice Computes the return of SwapToUnits.
     * @param from The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @return uint256 Group specific units.
     */
    function dry_swap_to_unit(
        address from,
        uint256 amount
    ) public view returns (uint256) {
        // A high => less units returned. Do not subtract the escrow amount
        uint256 A = IERC20(from).balanceOf(address(this));
        uint256 W = _weight[from];

        // If a token is not part of the pool, W is 0. This returns 0 since
        // 0^p = 0.
        return compute_integral(amount, A, W, _amp);
    }

    /**
     * @notice Computes the output of SwapFromUnits.
     * @param to The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function dry_swap_from_unit(
        address to, 
        uint256 U
    ) public view returns (uint256) {
        // A low => less tokens returned. Subtract the escrow amount to decrease the balance.
        uint256 B = IERC20(to).balanceOf(address(this)) - _escrowedTokens[to];
        uint256 W = _weight[to];

        // If someone were to purchase a token which is not part of the pool on setup
        // they would just add value to the pool. We don't care about it.
        // However, it will revert since the solved integral contains U/W and when
        // W = 0 then U/W returns division by 0 error.
        return solve_integral(U, B, W, _amp);
    }

    /**
     * @notice Computes the output of SwapToAndFromUnits.
     * @dev Implemented through solve_integral(compute_integral and not complete_integral.
     * @param from The address of the token to sell.
     * @param to The address of the token to buy.
     * @param amount The amount of from token to sell for to token.
     * @return Output denominated in to token.
     */
    function dry_swap_both(
        address from,
        address to,
        uint256 amount
    ) public view returns (uint256) {
        uint256 A = IERC20(from).balanceOf(address(this));
        uint256 B = IERC20(to).balanceOf(address(this)) - _escrowedTokens[to];
        uint256 W_A = _weight[from];
        uint256 W_B = _weight[to];
        uint256 amp = _amp;

        return solve_integral(compute_integral(amount, A, W_A, amp), B, W_B, amp);
    }

    /**
     * @notice Deposits a user configurable amount of tokens.
     * @dev Requires approvals for all tokens within the pool.
     * It is advised that the deposit matches the pool's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of pool tokens to be minted.
     */
    function depositMixed(
        uint256[] calldata tokenAmounts,
        uint256 minOut
    ) external returns(uint256) {
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);

        uint256 walpha_0_ampped;
        uint256 U;
        uint256 it;
        // First, lets derive walpha_0. This lets us evaluate the number of tokens
        // the pool should have If the price in the group is 1:1.
        {
            int256 intU = 0;
            // We don't need weightedAssetBalanceSum again.
            uint256 weightedAssetBalanceSum = 0;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // Not minus escrowedAmount, since we want the deposit to return less.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this)) * FixedPointMathLib.WAD;
                
                
                {
                    uint256 wab = uint256(FixedPointMathLib.powWad(int256(weightAssetBalance), oneMinusAmp));
                    weightedAssetBalanceSum += wab;

                    if (tokenAmounts[it] == 0) continue;
                    
                    intU -= int256(wab);
                }


                intU += FixedPointMathLib.powWad(int256(weightAssetBalance + weight * tokenAmounts[it] * FixedPointMathLib.WAD), oneMinusAmp);
                

                IERC20(token).safeTransferFrom(
                    msg.sender,
                    address(this),
                    tokenAmounts[it]
                );
            }
            U = uint256(intU);

            walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - int256(_unitTracker)) / it;
        }
        uint256 walpha_0 = uint256(FixedPointMathLib.powWad(
            int256(walpha_0_ampped),
            int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp
        )); // todo: Optimise?
        uint256 wpt_a = uint256(FixedPointMathLib.powWad(
            int256(walpha_0_ampped + U / it),
            int256(FixedPointMathLib.WAD*FixedPointMathLib.WAD / uint256(oneMinusAmp))
        )) - walpha_0;

        uint256 poolTokens = (wpt_a * totalSupply()) / walpha_0;
        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);
        // Mint the desired number of pool tokens to the user.
        _mint(msg.sender, poolTokens);

        emit Deposit(msg.sender, poolTokens, tokenAmounts);

        return poolTokens;
    }

    // @nonreentrant('lock')
    /**
     * @notice Deposits a symmetrical number of tokens such that in 1:1 wpt_a are deposited. This doesn't change the pool price.
     * @dev Requires approvals for all tokens within the pool.
     * @param poolTokens The number of pool tokens to mint.
     * @param minOut The minimum number of tokens minted.
     */
    function withdrawAll(uint256 poolTokens, uint256[] calldata minOut)
        external returns(uint256[] memory)
    {
        // Burn the desired number of pool tokens to the user. If they don't have it, it saves gas.
        _burn(msg.sender, poolTokens);

        // Cache weights and balances.
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        address[] memory tokenIndexed = new address[](NUMASSETS);
        uint256[] memory weightAssetBalances = new uint256[](NUMASSETS);
        uint256[] memory ampWeightAssetBalances = new uint256[](NUMASSETS);

        uint256 walpha_0_ampped;
        // First, lets derive walpha_0. This lets us evaluate the number of tokens the pool should have
        // If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            uint256 weightedAssetBalanceSum = 0;
            uint256 it;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                tokenIndexed[it] = token;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);
                weightAssetBalances[it] = weightAssetBalance; // Store since we need it later

                uint256 wab = uint256(FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),
                    oneMinusAmp)
                );
                ampWeightAssetBalances[it] = wab; // Store since it is an expensive calculation.
                weightedAssetBalanceSum += wab;
            }
            walpha_0_ampped = uint256((int256(weightedAssetBalanceSum) - int256(_unitTracker))) / it;
        }

        // For later event logging, the amounts transferred to the pool are stored.
        uint256[] memory amounts = new uint256[](NUMASSETS);
        // solve: poolTokens = (wpt_a * (totalSupply() + poolTokens))/walpha_0; for wpt_a.
        // Plus _escrowedPoolTokens since we want the withdrawal to return less.
        {
            uint256 innerdiff;
            { 
                // Remember to add the number of pool tokens burned to totalSupply
                uint256 ts = (totalSupply() + _escrowedPoolTokens + poolTokens);
                uint256 pt_fraction = ((ts + poolTokens) * FixedPointMathLib.WAD) / ts;
                innerdiff = FixedPointMathLib.mulWadUp(
                    walpha_0_ampped, 
                    uint256(FixedPointMathLib.powWad(int256(pt_fraction), oneMinusAmp)) - FixedPointMathLib.WAD);
            }
            for (uint256 it = 0; it < NUMASSETS; ++it) {
                address token = tokenIndexed[it];
                if (token == address(0)) break;

                uint256 tokenAmount = (uint256(FixedPointMathLib.powWad(
                    int256(ampWeightAssetBalances[it] + innerdiff),
                    int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp
                )) - (weightAssetBalances[it] * FixedPointMathLib.WAD)) / FixedPointMathLib.WAD;

                uint256 weight = _weight[token];
                if (tokenAmount > weightAssetBalances[it]) {
                    //! If the pool doesn't have enough assets for a withdrawal, then
                    //! withdraw all of the pools assets. This should be protected against by setting minOut != 0.
                    //! This is possible because the pool expects assets to come back. (it is owed assets)
                    tokenAmount = weightAssetBalances[it] / weight;
                    require(
                        tokenAmount >= minOut[it],
                        SWAP_RETURN_INSUFFICIENT
                    );
                    // Transfer the appropriate number of tokens from the user to the pool. (And store for event logging)
                    amounts[it] = tokenAmount;
                    IERC20(token).safeTransfer(msg.sender, tokenAmount); // dev: User doesn't have enough tokens or low approval
                    continue;
                }
                tokenAmount /= weight;
                require(tokenAmount >= minOut[it], SWAP_RETURN_INSUFFICIENT);
                // Transfer the appropriate number of tokens from the user to the pool. (And store for event logging)
                amounts[it] = tokenAmount;
                IERC20(token).safeTransfer(msg.sender, tokenAmount);
            }
        }

        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

    // @nonreentrant('lock')
    /**
     * @notice Deposits a symmetrical number of tokens such that in 1:1 wpt_a are deposited. This doesn't change the pool price.
     * @dev Requires approvals for all tokens within the pool.
     * @param poolTokens The number of pool tokens to withdraw
     * @param withdrawRatioX64 The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is X64
     * @param minOuts The minimum number of tokens minted.
     */
    function withdrawMixed(
        uint256 poolTokens,
        uint256[] calldata withdrawRatioX64,
        uint256[] calldata minOuts
    ) external returns(uint256[] memory) {
        // Here the implementation should cache: "totalSupply() + _escrowedPoolTokens".
        // However, there is not enough stack to do so. So when accessing totalSupply()
        // remember to add poolTokens to it. (As they are burned in the next line)
        // This implementation only access totalSupply once, so caching is not needed.

        // Burn the desired number of pool tokens to the user. If they don't have it, it saves gas.
        _burn(msg.sender, poolTokens);

        // Cache weights and balances.
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        uint256[] memory assetBalances = new uint256[](NUMASSETS);
        uint256[] memory ampWeightAssetBalances = new uint256[](NUMASSETS);

        uint256 U;
        // First, lets derive walpha_0. This lets us evaluate the number of tokens the pool should have
        // If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            uint256 it;
            uint256 walpha_0_ampped;
            {
                uint256 weightedAssetBalanceSum = 0;
                for (it = 0; it < NUMASSETS; ++it) {
                    address token = _tokenIndexing[it];
                    if (token == address(0)) break;
                    uint256 weight = _weight[token];

                    // minus escrowedAmount, since we want the withdrawal to return less.
                    uint256 ab = (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);
                    assetBalances[it] = ab;
                    uint256 weightAssetBalance = weight * ab;

                    uint256 wab = uint256(FixedPointMathLib.powWad(
                        int256(weightAssetBalance * FixedPointMathLib.WAD),
                        oneMinusAmp
                    ));
                    ampWeightAssetBalances[it] = wab; // Store since it is an expensive calculation.
                    weightedAssetBalanceSum += wab;
                }
                walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - int256(_unitTracker)) / it;
            }
            // Remember to add the number of pool tokens burned to totalSupply
            uint256 ts = totalSupply() + _escrowedPoolTokens + poolTokens;
            uint256 pt_fraction = FixedPointMathLib.divWadDown(ts + poolTokens, ts);
            U = it * FixedPointMathLib.mulWadDown(
                walpha_0_ampped, 
                uint256(FixedPointMathLib.powWad(int256(pt_fraction), oneMinusAmp)) - FixedPointMathLib.WAD
            );
        }

        uint256[] memory amounts = new uint256[](NUMASSETS);
        for (uint256 it = 0; it < NUMASSETS; ++it) {
            if (U == 0) break;

            uint256 U_i = (U * withdrawRatioX64[it]) / FixedPointMathLib.WAD;
            if (U_i == 0) continue;
            U -= U_i;

            // uint256 tokenAmount = dry_swap_from_unit(_tokenIndexing[it], U_i);
            uint256 tokenAmount;
            {
                // W_B · B^(1-k) is repeated twice and requires 1 power.
                // As a result, we compute it and cache.
                uint256 W_BxBtoOMA = ampWeightAssetBalances[it];
                tokenAmount = assetBalances[it] * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
                    int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U_i, W_BxBtoOMA)),
                    int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD / uint256(oneMinusAmp)))
                )) / FixedPointMathLib.WAD;
            }
            require(minOuts[it] <= tokenAmount, SWAP_RETURN_INSUFFICIENT);
            IERC20(_tokenIndexing[it]).safeTransfer(msg.sender, tokenAmount);

            amounts[it] = tokenAmount;
        }

        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

    // @nonreentrant('lock')
    /**
     * @notice A swap between 2 assets which both are inside the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of _fromAsset the user wants to sell
     * @param minOut The minimum output of _toAsset the user wants.
     */
    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) external returns (uint256) {
        _A();
        uint256 fee = FixedPointMathLib.mulWadDown(amount, _poolFee);

        // Calculate the swap return value.
        uint256 out = dry_swap_both(fromAsset, toAsset, amount - fee);

        // Check if the calculated returned value is more than the minimum output.
        require(out >= minOut, SWAP_RETURN_INSUFFICIENT);

        // Swap tokens with the user.
        IERC20(toAsset).safeTransfer(msg.sender, out);
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // TODO: Security limit update?

        emit LocalSwap(msg.sender, fromAsset, toAsset, amount, out);

        return out;
    }

    function swapToUnits(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        bytes memory calldata_
    ) public returns (uint256) {
        require(fallbackUser != address(0));
        _A();
        // uint256 fee = FixedPointMathLib.mulWadDown(amount, _poolFee);

        // Calculate the group specific units bought.
        uint256 U = dry_swap_to_unit(
            fromAsset,
            amount - FixedPointMathLib.mulWadDown(amount, _poolFee)
        );

        // Track units for fee distribution.
        _unitTracker += int128(uint128(U));

        // Compute the change in max_unit_inflow.
        {
            uint256 fromBalance = ERC20(fromAsset).balanceOf(address(this));
            _max_unit_inflow += FixedPointMathLib.mulWadUp(
                _ampUnitCONSTANT,
                getModifyUnitCapacity(
                    fromBalance - (amount - FixedPointMathLib.mulWadDown(amount, _poolFee)),
                    fromBalance,
                    fromAsset
                )
            );
        }

        bytes32 messageHash;

        {
            TokenEscrow memory escrowInformation = TokenEscrow({
                amount: amount - FixedPointMathLib.mulWadDown(amount, _poolFee),
                token: fromAsset
            });

            // Send the purchased units to targetPool on chain.
            messageHash = CatalystIBCInterface(_chaininterface).crossChainSwap(
                channelId,
                targetPool,
                targetUser,
                toAssetIndex,
                U,
                minOut,
                escrowInformation,
                calldata_
            );
        }

        // Collect the tokens from the user.
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // Escrow the tokens
        // ! Reentrancy. It is not possible to abuse the reentry, since the storage change is checked for validity first.
        require(_escrowedFor[messageHash] == address(0)); // User cannot supply fallbackUser = address(0)
        _escrowedTokens[fromAsset] += amount - FixedPointMathLib.mulWadDown(amount, _poolFee);
        _escrowedFor[messageHash] = fallbackUser;


        {
            // Governance Fee
            uint256 governanceFee = _governanceFee;
            if (governanceFee != 0) {
                uint256 governancePart = FixedPointMathLib.mulWadDown(
                    FixedPointMathLib.mulWadDown(amount, _poolFee),
                    governanceFee
                );
                IERC20(fromAsset).safeTransfer(factoryOwner(), governancePart);
            }
        }

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit at low cost.

        emit SwapToUnits(
            targetPool,
            targetUser,
            fromAsset,
            toAssetIndex,
            amount,
            U,
            minOut,
            messageHash
        );

        return U;
    }


    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another pool.
     * @param channelId The target chain identifier.
     * @param targetPool The target pool on the target chain encoded in bytes32. For EVM chains this can be computed as:
     * Vyper: convert(_poolAddress, bytes32)
     * Solidity: abi.encode(_poolAddress)
     * Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * @param targetUser The recipient of the transaction on _chain. Encoded in bytes32. For EVM chains it can be derived similarly to targetPool.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param amount The number of _fromAsset to sell to the pool.
     * @param minOut The minimum number of returned tokens to the targetUser on the target chain.
     * @param fallbackUser If the transaction fails send the escrowed funds to this address
     */
    function swapToUnits(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256) {
        bytes memory calldata_ = new bytes(0);
        return
            swapToUnits(
                channelId,
                targetPool,
                targetUser,
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
     * Called exclusively by the chaininterface.
     * @dev Can only be called by the chaininterface, as there is no way to check the
     * validity of units.
     * @param toAssetIndex Index of the asset to be purchased with _U units.
     * @param who The recipient of toAsset
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     */
    function swapFromUnits(
        uint256 toAssetIndex,
        address who,
        uint256 U,
        uint256 minOut,
        bytes32 messageHash
    ) public returns (uint256) {
        _A();
        // The chaininterface is the only valid caller of this function, as there cannot
        // be a check of U. (It is purely a number)
        require(msg.sender == _chaininterface);
        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];

        // Check if the swap is according to the swap limits
        checkAndSetUnitCapacity(U);

        // Calculate the swap return value.
        uint256 purchasedTokens = dry_swap_from_unit(toAsset, U);

        require(minOut <= purchasedTokens, SWAP_RETURN_INSUFFICIENT);


        // Track units for fee distribution.
        _unitTracker -= int128(uint128(U));

        // Compute the change in max_unit_inflow.
        uint256 toBalance = IERC20(toAsset).balanceOf(address(this)) -
            _escrowedTokens[toAsset] - purchasedTokens;
        _max_unit_inflow -= FixedPointMathLib.mulWadUp(
            _ampUnitCONSTANT,
            getModifyUnitCapacity(
                toBalance + purchasedTokens,
                toBalance,
                toAsset
            )
        );

        // Send the return value to the user.
        IERC20(toAsset).safeTransfer(who, purchasedTokens);

        emit SwapFromUnits(who, toAsset, U, purchasedTokens, messageHash);

        return purchasedTokens; // Unused.
    }

    function swapFromUnits(
        uint256 toAssetIndex,
        address who,
        uint256 U,
        uint256 minOut,
        bytes32 messageHash,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256) {
        uint256 purchasedTokens = swapFromUnits(
            toAssetIndex,
            who,
            U,
            minOut,
            messageHash
        );

        ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, data);

        return purchasedTokens;
    }

    //--- Liquidity swapping ---//
    // Because of the way pool tokens work in a group of pools, there
    // needs to be a way to manage an equilibrium between pool token
    // value and token pool value. Liquidity swaps is a macro implemented
    // on the smart contract level to:
    // 1. Withdraw tokens.
    // 2. Convert tokens to units & transfer to target pool.
    // 3. Convert units to an even mix of tokens.
    // 4. Deposit the even mix of tokens.
    // In 1 user invocation.

    // @nonreentrant('lock')
    /**
     * @notice Initiate a cross-chain liquidity swap by lowering liquidity and transfer the liquidity units to another pool.
     * @param channelId The target chain identifier.
     * @param targetPool The target pool on the target chain encoded in bytes32. For EVM chains this can be computed as:
     * Vyper: convert(_poolAddress, bytes32)
     * Solidity: abi.encode(_poolAddress)
     * Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * @param targetUser The recipient of the transaction on _chain. Encoded in bytes32. For EVM chains it can be found similarly to _targetPool.
     * @param poolTokens The number of pool tokens to liquidity Swap
     */
    function outLiquidity(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        uint256 poolTokens,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256) {
        require(fallbackUser != address(0));
        _A();

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, poolTokens);

        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        uint256 walpha_0_ampped;
        uint256 it;
        // First, lets derive walpha_0. This lets us evaluate the number of tokens the pool should have
        // If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            uint256 weightedAssetBalanceSum = 0;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);

                weightedAssetBalanceSum += uint256(FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),
                    oneMinusAmp
                ));
            }
            walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - int256(_unitTracker)) / it;
        }

        uint256 U = 0;
        {
            uint256 walpha_0 = uint256(FixedPointMathLib.powWad(
                int256(walpha_0_ampped),
                int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp
            ));
            // Plus _escrowedPoolTokens since we want the withdrawal to return less.
            U = it * uint256(FixedPointMathLib.powWad(
                int256((walpha_0 * poolTokens)/(totalSupply() + _escrowedPoolTokens + poolTokens)),
                oneMinusAmp
            ));
            //TODO: Optimise
        }

        bytes32 messageHash;
        {
            LiquidityEscrow memory escrowInformation = LiquidityEscrow({
                poolTokens: poolTokens
            });

            // Sending the liquidity units over.
            messageHash = CatalystIBCInterface(_chaininterface).liquiditySwap(
                channelId,
                targetPool,
                targetUser,
                U,
                minOut,
                escrowInformation
            );
        }

        // ! Reentrancy. It is not possible to abuse the reentry, since the storage change is checked for validity first.
        // Escrow the pool tokens
        require(_escrowedLiquidityFor[messageHash] == address(0));
        _escrowedLiquidityFor[messageHash] = fallbackUser;
        _escrowedPoolTokens += poolTokens;

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit at low cost.
        emit SwapToLiquidityUnits(
            targetPool,
            targetUser,
            poolTokens,
            U,
            messageHash
        );

        return U;
    }


    // @nonreentrant('lock')
    /**
     * @notice Completes a cross-chain swap by converting liquidity units to pool tokens
     * Called exclusively by the chaininterface.
     * @dev Can only be called by the chaininterface, as there is no way
     * to check the validity of units.
     * @param who The recipient of pool tokens
     * @param U Number of units to convert into pool tokens.
     */
    function inLiquidity(
        address who,
        uint256 U,
        uint256 minOut,
        bytes32 messageHash
    ) external returns (uint256) {
        _A();
        // The chaininterface is the only valid caller of this function, as there cannot
        // be a check of _U. (It is purely a number)
        require(msg.sender == _chaininterface);

        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        uint256 walpha_0_ampped;
        uint256 it;
        // First, lets derive walpha_0. This lets us evaluate the number of tokens the pool should have
        // If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            uint256 weightedAssetBalanceSum = 0;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);

                weightedAssetBalanceSum += uint256(FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),
                    oneMinusAmp
                ));
            }
            walpha_0_ampped = uint256((int256(weightedAssetBalanceSum) - int256(_unitTracker))) / it;
        }

        uint256 ts = totalSupply(); // Not! + _escrowedPoolTokens, since a smaller supply results in fewer pool tokens.

        uint256 walpha_0 = uint256(FixedPointMathLib.powWad(int256(walpha_0_ampped), int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp));
        uint256 wpt_a = it * uint256(FixedPointMathLib.powWad(
            int256(walpha_0_ampped + U / it),
            int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp
        )) - walpha_0;
        uint256 poolTokens = (wpt_a * ts) / walpha_0;

        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);

        // Mint pool tokens for _who
        _mint(who, poolTokens);

        emit SwapFromLiquidityUnits(who, U, poolTokens, messageHash);

        return poolTokens;
    }
}
