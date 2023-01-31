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
 * swappool contract also defines the pool token.
 * @dev This contract is deployed inactive: It cannot be used as a
 * swap pool as is. To use it, a proxy contract duplicating the
 * logic of this contract needs to be deployed. In Vyper, this
 * can be done through (vy >=0.3.4) create_minimal_proxy_to.
 * In Solidity, this can be done through OZ clones: Clones.clone(...)
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
    // When the swap is a very small size of the pool, the swaps
    // returns slightly more. To counteract this, an additional fee
    // slightly larger than the error is added. The below constants
    // determines when this fee is added and the size.
    uint256 constant SMALL_SWAP_SIZE = 10**12;
    uint256 constant SMALL_SWAP_RETURN = 95*10**18/100;

    // For other config options, see SwapPoolCommon.sol

    //-- Variables --//
    uint256 public _amp;
    uint256 public _targetAmplification;

    // To keep track of group ownership, the pool needs to keep track of
    // the local unit balance. That is, do other pools own or owe assets to this pool?
    int256 public _unitTracker;

    /**
     * @notice Configures an empty pool.
     * @dev If less than NUMASSETS are used to initiate the pool
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
     * If set to values with low resolution (<= 10*5), this should be viewed as
     * opt out of governance weight adjustment. This is not enforced.
     * @param amp Amplification factor. Should be < 10**18.
     * @param depositor The address depositing the initial token balances.
     */
    function initializeSwapCurves(
        address[] calldata init_assets,
        uint256[] calldata weights,
        uint256 amp,
        address depositor
    ) public {
        require(msg.sender == _factory && _tokenIndexing[0] == address(0));  // dev: swap curves may only be initialized once by the factory
        // Check that the amplification is correct.
        require(amp < FixedPointMathLib.WAD);  // dev: Amplification not set correctly.
        // Check for a misunderstanding regarding how many assets this pool supports.
        require(init_assets.length <= NUMASSETS);  // dev: Too many pool supplies for this pool template.

        _amp = amp;
        _targetAmplification = amp;

        // Compute the security limit.
        { //  Stack limitation.
            uint256[] memory initialBalances = new uint256[](NUMASSETS);
            for (uint256 it = 0; it < init_assets.length; ++it) {
                address tokenAddress = init_assets[it];
                _tokenIndexing[it] = tokenAddress;
                uint256 weight = weights[it];
                _weight[tokenAddress] = weight;
                // The contract expects the tokens to have been sent to it before setup is
                // called. Make sure the pool has more than 0 tokens.

                uint256 balanceOfSelf = IERC20(tokenAddress).balanceOf(address(this));
                initialBalances[it] = balanceOfSelf;
                require(balanceOfSelf > 0); // dev: 0 tokens provided in setup.

                _max_unit_inflow += weight * balanceOfSelf;
            }
            
            emit Deposit(depositor, MINTAMOUNT, initialBalances);
        }

        // Mint pool tokens for pool creator.
        _mint(depositor, MINTAMOUNT);
    }

    /** 
     * @notice  Returns the current cross-chain swap capacity. 
     * @dev Overwrites the common implementation because of the
     * differences as to how it is used. As a result, this always returns
     * half of the true limit.
     */
    function getUnitCapacity() external view override returns (uint256) {
        uint256 MUF = _max_unit_inflow;
        // If the time since the last update is more than DECAYRATE return maximum
        if (block.timestamp > DECAYRATE + _last_change) return MUF / 2;

        // The delta change to the limit is: timePassed · slope = timePassed · Max/decayrate
        uint256 delta_flow = ((block.timestamp - _last_change) * MUF) / DECAYRATE;

        uint256 UF = _unit_flow;
        // If the change is greater than the units which have passed through
        // return maximum. We do not want (MUF - (UF - delta_flow) > MUF)
        if (UF <= delta_flow) return MUF / 2;

        // Amplified pools can have MUF <= UF since MUF is modified when swapping
        if (MUF <= UF - delta_flow) return 0; 

        return (MUF + delta_flow - UF) / 2;
    }

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
        uint256 U = compute_integral(amount, A, W, _amp);

        // If the swap is a very small portion of the pool
        // Add an additional fee. This covers mathematical errors.
        if (A/SMALL_SWAP_SIZE >= amount) return U * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;
        
        return U;
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

        // return complete_integral(amount, A, B, W_A, W_B, amp);
        uint256 output = solve_integral(compute_integral(amount, A, W_A, amp), B, W_B, amp);

        // If the swap is a very small portion of the pool
        // Add an additional fee. This covers mathematical errors.
        if (A/SMALL_SWAP_SIZE >= amount) return output * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;

        return output;
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
    ) nonReentrant() external returns(uint256) {
        _A();
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);

        uint256 walpha_0_ampped;

        // There is a Stack too deep issue in a later branch. To counteract this,
        // wab is stored short-lived. This requires letting U get negative.
        // As such, we define an additional variable called intU which is signed
        int256 U;
        uint256 it;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            uint256 weightedAssetBalanceSum = 0;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // Not minus escrowedAmount, since we want the deposit to return less.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this));

                // Increase the security limit by the amount deposited.
                _max_unit_inflow += weightAssetBalance;
                // Short term decrease the security limit by the amount deposited.
                _unit_flow += weightAssetBalance;
                
                {
                    // wa^(1-k) is required twice. It is F(A) in the
                    // swapToUnits equation and part of the wa_0^(1-k) calculation
                    uint256 wab = uint256(FixedPointMathLib.powWad(
                        int256(weightAssetBalance * FixedPointMathLib.WAD),
                        oneMinusAmp
                    ));
                    weightedAssetBalanceSum += wab;

                    // This line is the origin of the stack too deep issue.
                    // since it implies we cannot move intU += before this section.
                    // which would solve the issue.
                    // Save gas if the user provides no tokens.
                    if (tokenAmounts[it] == 0) continue;
                    
                    // int_A^{A+x} f(w) dw = F(A+x) - F(A).
                    // This is -F(A). Since we are subtracting first,
                    // U must be able to go negative.
                    U -= int256(wab);
                }

                // Add F(A+x)
                U += FixedPointMathLib.powWad(
                    int256((weightAssetBalance + weight * tokenAmounts[it]) * FixedPointMathLib.WAD),
                    oneMinusAmp
                );
                
                IERC20(token).safeTransferFrom(
                    msg.sender,
                    address(this),
                    tokenAmounts[it]
                );  // dev: Token withdrawal from user failed.
            }

            // Compute the reference liquidity.
            walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - _unitTracker) / it;
        }

        // Subtract fee from U. This stops people from using deposit and withdrawal as method of swapping.
        // To reduce costs, there the governance fee is not included. This does result in deposit+withdrawal
        // working as a way to circumvent the governance fee.
        U = int256(FixedPointMathLib.mulWadDown(uint256(U), FixedPointMathLib.WAD - _poolFee));

        // Use the simplified conversion of units into pool tokens. Saves ~1000 gas.
        uint256 it_times_walpha_amped = it * walpha_0_ampped;
        uint256 poolTokens = (totalSupply() * uint256(FixedPointMathLib.powWad(
            int256(FixedPointMathLib.divWadDown(
                it_times_walpha_amped + uint256(U),
                it_times_walpha_amped
            )),
            int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp
        ) - int256(FixedPointMathLib.WAD))) / FixedPointMathLib.WAD;

        // Check if the return satisfies the user.
        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);

        // Mint the desired number of pool tokens to the user.
        _mint(msg.sender, poolTokens);

        emit Deposit(msg.sender, poolTokens, tokenAmounts);

        return poolTokens;
    }

    /**
     * @notice Burns poolTokens and releases the symmetrical share of tokens to the burner. 
     * This doesn't change the pool price.
     * @dev This is the cheapest way to withdraw.
     * @param poolTokens The number of pool tokens to burn.
     * @param minOut The minimum token output. If less is returned, the tranasction reverts.
     */
    function withdrawAll(
        uint256 poolTokens,
        uint256[] calldata minOut
    ) nonReentrant() external returns(uint256[] memory) {
        _A();
        // Burn the desired number of pool tokens to the user.
        // If they don't have it, it saves gas.
        // * Remember to add poolTokens when accessing totalSupply()
        _burn(msg.sender, poolTokens);

        // Cache weights and balances.
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        address[] memory tokenIndexed = new address[](NUMASSETS);
        uint256[] memory weightAssetBalances = new uint256[](NUMASSETS);
        uint256[] memory ampWeightAssetBalances = new uint256[](NUMASSETS);

        uint256 walpha_0_ampped;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            uint256 weightedAssetBalanceSum = 0;
            // "it" is needed breifly outside the loop.
            uint256 it;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                tokenIndexed[it] = token;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);
                weightAssetBalances[it] = weightAssetBalance; // Store 

                uint256 wab = uint256(FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),
                    oneMinusAmp)
                );
                ampWeightAssetBalances[it] = wab; // Store
                weightedAssetBalanceSum += wab;
            }

            // Compute the reference liquidity.
            walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) -_unitTracker) / it;
        }

        // For later event logging, the transferred tokens are stored.
        uint256[] memory amounts = new uint256[](NUMASSETS);
        {
            // wtk = (wa^(1-k) + (wa_0 + wpt)^(1-k) - wa_0^(1-k)))^(1/(1-k)) - wa
            // The inner diff is (wa_0 + wpt)^(1-k) - wa_0^(1-k).
            // since it doesn't depend on the token, it should only be computed once
            // The following is a reduction of the equation to reduce costs.
            uint256 innerdiff;
            { 
                // Remember to add the number of pool tokens burned to totalSupply
                // _escrowedPoolTokens is added, since it makes makes pt_fraction smaller
                uint256 ts = (totalSupply() + _escrowedPoolTokens + poolTokens);
                uint256 pt_fraction = ((ts + poolTokens) * FixedPointMathLib.WAD) / ts;

                // The reduced equation:
                innerdiff = FixedPointMathLib.mulWadDown(
                    walpha_0_ampped, 
                    uint256(FixedPointMathLib.powWad(
                        int256(pt_fraction),
                        oneMinusAmp
                    )) - FixedPointMathLib.WAD
                );
            }
            // While not very readable, reusing this variable makes a lot of sense.
            // So from now one, oneMinusAmp is 1/oneMinusAmp
            oneMinusAmp = int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp;

            for (uint256 it = 0; it < NUMASSETS; ++it) {
                address token = tokenIndexed[it];
                if (token == address(0)) break;

                // wtk = (wa^(1-k) + (wa_0 + wpt)^(1-k) - wa_0^(1-k)))^(1/(1-k)) - wa
                // wtk = (wa^(1-k) + innerDiff)^(1/(1-k)) - wa
                uint256 tokenAmount = (uint256(FixedPointMathLib.powWad(
                    int256(ampWeightAssetBalances[it] + innerdiff),
                    oneMinusAmp // Is 1/(1-amp)
                )) - (weightAssetBalances[it] * FixedPointMathLib.WAD)) / FixedPointMathLib.WAD;

                // ideally, we would have cached this. But stack limit :|
                uint256 weight = _weight[token];
                //! If the pool doesn't have enough assets for a withdrawal, then
                //! withdraw all of the pools assets. This should be protected against by setting minOut != 0.
                //! This happens because the pool expects assets to come back. (it is owed assets)
                //! We don't want to keep track of debt so we simply return less
                if (tokenAmount > weightAssetBalances[it]) {
                    // Set the token amount to the pool balance.
                    // Recalled that the escrow balance is subtracted from weightAssetBalances.
                    tokenAmount = weightAssetBalances[it] / weight;

                    // Decrease the security limit by the amount withdrawn.
                    _max_unit_inflow -=  weightAssetBalances[it]; // = tokenAmount * weight
                    if (_unit_flow <= weightAssetBalances[it]) {
                        _unit_flow = 0;
                    } else {
                        _unit_flow -= weightAssetBalances[it];
                    }

                    // Check that the user is satisfied with this.
                    require(
                        tokenAmount >= minOut[it],
                        SWAP_RETURN_INSUFFICIENT
                    );

                    // Transfer the appropriate number of tokens from the user to the pool. (And store for event logging)
                    amounts[it] = tokenAmount;
                    IERC20(token).safeTransfer(msg.sender, tokenAmount); // dev: Transfer away from pool failed.
                    continue;
                }
                // Decrease the security limit by the amount withdrawn.
                _max_unit_inflow -=  tokenAmount;
                if (_unit_flow <= tokenAmount) {
                    _unit_flow = 0;
                } else {
                    _unit_flow -= tokenAmount;
                }

                // remove the weight from tokenAmount.
                tokenAmount /= weight;
                // Check that the user is satisfied with this.
                require(tokenAmount >= minOut[it], SWAP_RETURN_INSUFFICIENT);
                // Transfer the appropriate number of tokens from the user to the pool. (And store for event logging)
                amounts[it] = tokenAmount;
                IERC20(token).safeTransfer(msg.sender, tokenAmount);
            }
        }

        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

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
    ) nonReentrant() external returns(uint256[] memory) {
        _A();
        // Burn the desired number of pool tokens to the user.
        // If they don't have it, it saves gas.
        // * Remember to add poolTokens when accessing totalSupply()
        _burn(msg.sender, poolTokens);

        // Cache weights and balances.
        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        address[] memory tokenIndexed = new address[](NUMASSETS);
        uint256[] memory assetBalances = new uint256[](NUMASSETS);
        uint256[] memory ampWeightAssetBalances = new uint256[](NUMASSETS);

        uint256 U = 0;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        // unlike in withdrawAll, this value is only needed to compute U.
        {
            // As such, we don't need to remember the value beyond this section.
            uint256 walpha_0_ampped;
            {
                uint256 weightedAssetBalanceSum = 0;
                // A very carefuly stack optimisation is made here.
                // "it" is needed breifly outside the loop. However, to reduce the number
                // of items in the stack, U = it.
                for (U = 0; U < NUMASSETS; ++U) {
                    address token = _tokenIndexing[U];
                    if (token == address(0)) break;
                    tokenIndexed[U] = token;
                    uint256 weight = _weight[token];

                    // minus escrowedAmount, since we want the withdrawal to return less.
                    uint256 ab = (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);
                    assetBalances[U] = ab;
                    uint256 weightAssetBalance = weight * ab;

                    uint256 wab = uint256(FixedPointMathLib.powWad(
                        int256(weightAssetBalance * FixedPointMathLib.WAD),
                        oneMinusAmp
                    ));
                    ampWeightAssetBalances[U] = wab; // Store since it is an expensive calculation.
                    weightedAssetBalanceSum += wab;
                }
                walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - _unitTracker) / U;

                // set U = number of tokens in the pool. But that is exactly what it is.
            }
            // Remember to add the number of pool tokens burned to totalSupply
            uint256 ts = totalSupply() + _escrowedPoolTokens + poolTokens;
            uint256 pt_fraction = FixedPointMathLib.divWadDown(ts + poolTokens, ts);

            U *= FixedPointMathLib.mulWadDown(
                walpha_0_ampped, 
                uint256(FixedPointMathLib.powWad(int256(pt_fraction), oneMinusAmp)) - FixedPointMathLib.WAD
            );
        }

        // While not very readable, reusing this variable makes a lot of sense.
        // So from now one, oneMinusAmp is 1/oneMinusAmp
        oneMinusAmp = int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp;

        // For later event logging, the transferred tokens are stored.
        uint256[] memory amounts = new uint256[](NUMASSETS);
        for (uint256 it = 0; it < NUMASSETS; ++it) {
            if (U == 0) break;

            uint256 U_i = (U * withdrawRatioX64[it]) / FixedPointMathLib.WAD;
            if (U_i == 0) continue;
            U -= U_i;

            // uint256 tokenAmount = dry_swap_from_unit(_tokenIndexing[it], U_i);
            
            // W_B · B^(1-k) is repeated twice and requires 1 power.
            // As a result, we compute it and cache.
            uint256 W_BxBtoOMA = ampWeightAssetBalances[it];
            uint256 tokenAmount = (assetBalances[it] * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
                int256(FixedPointMathLib.divWadUp(W_BxBtoOMA - U_i, W_BxBtoOMA)),
                oneMinusAmp // Is 1/(1-amp))
            )))) / FixedPointMathLib.WAD;
            
            // We need to check that the withdrawal doesn't impact any escrowed balances.
            // For this, we need a seperate check.
            require(assetBalances[it] >= tokenAmount); // dev: Pool balance too low.
            // Check that the user is satisfied with this.
            require(minOuts[it] <= tokenAmount, SWAP_RETURN_INSUFFICIENT);
            // Transfer the appropriate number of tokens from the user to the pool. (And store for event logging)
            amounts[it] = tokenAmount;
            IERC20(tokenIndexed[it]).safeTransfer(msg.sender, tokenAmount);

            // Decrease the security limit by the amount withdrawn.
            uint256 weight = _weight[tokenIndexed[it]];
            tokenAmount *= weight;
            _max_unit_inflow -=  tokenAmount;
            if (_unit_flow <= tokenAmount) {
                _unit_flow = 0;
            } else {
                _unit_flow -= tokenAmount;
            }
        }

        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

    /**
     * @notice A swap between 2 assets within the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of toAsset the user wants.
     */
    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) nonReentrant() external returns (uint256) {
        _A();
        uint256 fee = FixedPointMathLib.mulWadDown(amount, _poolFee);

        // Calculate the swap return value.
        uint256 out = dry_swap_both(fromAsset, toAsset, amount - fee);

        // Check if the calculated returned value is more than the minimum output.
        require(out >= minOut, SWAP_RETURN_INSUFFICIENT);

        IERC20(toAsset).safeTransfer(msg.sender, out);
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // For amplified pools, the security limit is based on the sum of the tokens
        // in the pool.
        if (out > amount) {
            _max_unit_inflow -= out - amount;
        } else {
            _max_unit_inflow += amount - out;
        }

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

        // Track units for computing balance0.
        _unitTracker += int256(U);

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


        // Escrow the tokens
        require(_escrowedFor[messageHash] == address(0)); // dev: Escrow already exists.
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

        // Collect the tokens from the user.
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

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
     * @dev Can only be called by the chaininterface, as there is no way to check validity of units.
     * @param toAssetIndex Index of the asset to be purchased with _U units.
     * @param who The recipient of toAsset
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     * @param messageHash Used to connect 2 swaps within a group. 
     */
    function swapFromUnits(
        uint256 toAssetIndex,
        address who,
        uint256 U,
        uint256 minOut,
        bytes32 messageHash
    ) public returns (uint256) {
        // The chaininterface is the only valid caller of this function, as there cannot
        // be a check of U. (It is purely a number)
        require(msg.sender == _chaininterface);
        _A();

        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];


        // Calculate the swap return value.
        uint256 purchasedTokens = dry_swap_from_unit(toAsset, U);


        // Check if the swap is according to the swap limits
        uint256 deltaSecurityLimit = purchasedTokens * _weight[toAsset];
        _max_unit_inflow -= deltaSecurityLimit;
        checkAndSetUnitCapacity(deltaSecurityLimit);

        require(minOut <= purchasedTokens, SWAP_RETURN_INSUFFICIENT);

        // Track units for fee distribution.
        _unitTracker -= int256(U);

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

        // Let users define custom logic which should be executed after the swap.
        // The logic is not contained within a try - except so if the logic reverts
        // the transaction will timeout. If this is not desired, wrap further logic
        // in a try - except at dataTarget.
        ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, data);

        return purchasedTokens;
    }

    //--- Liquidity swapping ---//
    // Because of the way pool tokens work in a group of pools, there
    // needs to be a way for users to easily get a distributed stake.
    // Liquidity swaps is a macro implemented  on the smart contract level to:
    // 1. Withdraw tokens.
    // 2. Convert tokens to units & transfer to target pool.
    // 3. Convert units to an even mix of tokens.
    // 4. Deposit the even mix of tokens.
    // In 1 user invocation.

    /**
     * @notice Initiate a cross-chain liquidity swap by lowering liquidity
     * and transfer the liquidity units to another pool.
     * @dev No reentry protection since only trusted contracts are called.
     * @param channelId The target chain identifier.
     * @param targetPool The target pool on the target chain encoded in bytes32.
     * @param targetUser The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param poolTokens The number of pool tokens to exchange
     * @param minOut The minimum number of pool tokens to mint on target pool.
     */
    function outLiquidity(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        uint256 poolTokens,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256) {
        // There needs to be provided a valid fallbackUser.
        require(fallbackUser != address(0));
        _A();

        _burn(msg.sender, poolTokens);

        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        uint256 walpha_0_ampped;
        uint256 it;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            uint256 weightedAssetBalanceSum = 0;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // minus escrowedAmount, since we want the withdrawal to return less.
                // A smaller number here means less units are transfered.
                uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _escrowedTokens[token]);

                weightedAssetBalanceSum += uint256(FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),
                    oneMinusAmp
                ));
            }
            walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - _unitTracker) / it;
        }

        uint256 U = 0;
        {
            // Plus _escrowedPoolTokens since we want the withdrawal to return less.
            uint256 ts = totalSupply() + _escrowedPoolTokens + poolTokens;
            uint256 pt_fraction = FixedPointMathLib.divWadDown(ts + poolTokens, ts);

            U = it * FixedPointMathLib.mulWadDown(
                walpha_0_ampped, 
                uint256(FixedPointMathLib.powWad(int256(pt_fraction), oneMinusAmp)) - FixedPointMathLib.WAD
            );
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

    /**
     * @notice Completes a cross-chain swap by converting liquidity units to pool tokens
     * @dev No reentry protection since only trusted contracts are called.
     * Called exclusively by the chaininterface.
     * @param who The recipient of pool tokens
     * @param U Number of units to convert into pool tokens.
     * @param minOut Minimum number of tokens to mint, otherwise reject.
     * @param messageHash Used to connect 2 swaps within a group. 
     */
    function inLiquidity(
        address who,
        uint256 U,
        uint256 minOut,
        bytes32 messageHash
    ) external returns (uint256) {
        // The chaininterface is the only valid caller of this function.
        require(msg.sender == _chaininterface);
        _A();

        int256 oneMinusAmp = int256(FixedPointMathLib.WAD - _amp);
        uint256 walpha_0_ampped;
        uint256 it;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the pool should have If the price in the group is 1:1.
        {
            // We don't need weightedAssetBalanceSum again.
            uint256 weightedAssetBalanceSum = 0;
            for (it = 0; it < NUMASSETS; ++it) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];

                // not minus escrowedAmount, since we want the withdrawal to return less.
                // A larger number here means more units have to be transfered.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this));

                weightedAssetBalanceSum += uint256(FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),
                    oneMinusAmp
                ));
            }
            walpha_0_ampped = uint256(int256(weightedAssetBalanceSum) - _unitTracker) / it;
        }

        // While not very readable, reusing this variable makes a lot of sense.
        // So from now one, oneMinusAmp is 1/oneMinusAmp
        oneMinusAmp = int256(FixedPointMathLib.WAD * FixedPointMathLib.WAD) / oneMinusAmp;

        uint256 ts = totalSupply(); // Not! + _escrowedPoolTokens, since a smaller supply results in fewer pool tokens.

        uint256 it_times_walpha_amped = it * walpha_0_ampped;
        uint256 poolTokens = (ts * (uint256(FixedPointMathLib.powWad(
            int256(FixedPointMathLib.divWadDown(
                it_times_walpha_amped + U,
                it_times_walpha_amped
            )),
            oneMinusAmp
        )) - FixedPointMathLib.WAD)) / FixedPointMathLib.WAD;

        // Check if the user would accept the mint.
        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);

        // Security limit
        {
            // To calculate the poolTokenEquiv, we set \alpha_t = \alpha_0.
            // This should be a close enough approximation.
            // If U > it_times_walpha_amped, then U can purchase more than 50% of the pool.
            // And the below calculation doesn't work.
            require(it_times_walpha_amped > U, EXCEEDS_SECURITY_LIMIT);
            uint256 poolTokenEquiv = FixedPointMathLib.mulWadUp(uint256(FixedPointMathLib.powWad(
                int256(it_times_walpha_amped),
                oneMinusAmp
            )), (FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
                int256(FixedPointMathLib.divWadDown(
                    it_times_walpha_amped - U,
                    it_times_walpha_amped
                )),
                oneMinusAmp
            ))));
            // Check if the swap is according to the swap limits
            checkAndSetUnitCapacity(poolTokenEquiv * 2 / FixedPointMathLib.WAD);
        }

        // Mint pool tokens for the user.
        _mint(who, poolTokens);

        emit SwapFromLiquidityUnits(who, U, poolTokens, messageHash);

        return poolTokens;
    }

    //-- Escrow Functions --//

    /** 
     * @notice Deletes and releases escrowed tokens to the pool.
     * @dev Should never revert!  
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     */
    function releaseEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external override {
        baseReleaseEscrowACK(messageHash, U, escrowAmount, escrowToken);

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UF = _unit_flow;
            // If UF < escrowAmount and we do UF - escrowAmount < 0 underflow => bad.
            if (UF > escrowAmount) {
                _unit_flow = UF - escrowAmount; // Does not underflow since _unit_flow > escrowAmount.
            } else if (UF != 0) {
                // If UF == 0, then we shouldn't do anything. Skip that case.
                // when UF <= escrowAmount => UF - escrowAmount <= 0 => max(UF - escrowAmount, 0) = 0
                _unit_flow = 0;
            }
            _max_unit_inflow += escrowAmount * _weight[escrowToken];
        }
    }

    /** 
     * @notice Deletes and releases escrowed tokens to the user.
     * @dev Should never revert!  
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     */
    function releaseEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) external override {
        baseReleaseEscrowTIMEOUT(messageHash, U, escrowAmount, escrowToken);
    }

    /** 
     * @notice Deletes and releases liquidity escrowed tokens to the pool.
     * @dev Should never revert!
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     */
    function releaseLiquidityEscrowACK(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external override {
        baseReleaseLiquidityEscrowACK(messageHash, U, escrowAmount);

        // We are unable to increase the security limit in this case.
        // If someone liquidity swapped a significant amount of assets
        // it is assumed the pool has low liquidity. In these cases,
        // liquidity swaps shouldn't be used.
    }

    /** 
     * @notice Deletes and releases escrowed pools tokens to the user.
     * @dev Should never revert!
     * @param messageHash A hash of the cross-chain message used ensure the message arrives indentical to the sent message.
     * @param U The number of units initially acquired.
     * @param escrowAmount The number of pool tokens escrowed.
     */
    function releaseLiquidityEscrowTIMEOUT(
        bytes32 messageHash,
        uint256 U,
        uint256 escrowAmount
    ) external override {
        baseReleaseLiquidityEscrowTIMEOUT(messageHash, U, escrowAmount);
    }
}
