//SPDX-License-Identifier: Unlicsened

pragma solidity ^0.8.17;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./FixedPointMath.sol";
import "./CatalystIBCInterface.sol";
import "./SwapPoolCommon.sol";
import "./ICatalystV1Pool.sol";

/**
 * @title Catalyst: The Multi-Chain Swap pool
 * @author Catalyst Labs
 * @notice Catalyst multi-chain swap pool using the asset specific
 * pricing curve: W/(w · ln(2)) where W is an asset specific
 * weight and w is the pool balance.
 *
 * The following contract supports between 1 and 3 assets for
 * atomic swaps. To increase the number of tokens supported,
 * change NUMASSETS to the desired maximum token amount.
 *
 * Implements the ERC20 specification, such that the contract
 * will be its own pool token.
 * @dev This contract is deployed /broken/: It cannot be used as a
 * swap pool as is. To use it, a proxy contract duplicating the
 * logic of this contract needs to be deployed. In vyper, this
 * can be done through (vy 0.3.4) create_minimal_proxy_to.
 * After deployment of the proxy, call setup(...). This will
 * initialize the pool and prepare it for cross-chain transactions.
 *
 * If connected to a supported cross-chain interface, call
 * createConnection or createConnectionWithChain to connect the
 * pool with pools on other chains.
 *
 * Finally, call finishSetup to give up the deployer's control
 * over the pool.
 */
contract CatalystSwapPool is
    CatalystFixedPointMath,
    CatalystSwapPoolCommon
{
    using SafeERC20 for IERC20;

    //--- ERRORS ---//

    //--- Config ---//
    // The following section contains the configurable variables.

    //-- Variables --//

    /**
     * @notice Setup a pool.
     * @dev The @param amp is only used as a sanity check and needs to be set to 2**64.
     * If less than NUMASSETS are used to setup the pool, let the remaining init_assets be ZERO_ADDRESS
     * The unused weights can be whatever. (however, 0 is recommended.)
     * The initial token amounts should have been sent to the pool before setup.
     * If any token has token amount 0, the pool will never be able to have more than
     * 0 tokens for that token.
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
        require(amp == ONE);
        require(init_assets.length <= NUMASSETS);
        _governanceFee = governanceFee;
        {
            uint256 max_unit_inflow = 0;
            for (uint256 it = 0; it < init_assets.length; it++) {
                address tokenAddress = init_assets[it];
                _tokenIndexing[it] = tokenAddress;
                _weight[tokenAddress] = weights[it];
                // The contract expect the tokens to have been sent to it before setup is
                // called. Make sure the pool has more than 0 tokens.
                {
                    uint256 balanceOfSelf = IERC20(tokenAddress).balanceOf(
                        address(this)
                    );
                    require(balanceOfSelf > 0); // dev: 0 tokens provided in setup.
                }

                // The maximum unit flow is \sum Weights. The value is shifted 64
                // since units are always X64.
                max_unit_inflow += weights[it] << 64;
            }
            _max_unit_inflow = max_unit_inflow;
        }

        setupBase(name_, symbol_, chaininterface, setupMaster);
    }

    function modifyWeights(uint256 targetTime, uint256[] calldata newWeights)
        external
        onlyFactoryOwner
    {
        require(targetTime >= block.timestamp + 60 * 60 * 24 * 2); // dev: targetTime must be more than 2 days in the future.
        _adjustmentTarget = targetTime;
        _lastModificationTime = block.timestamp;
        uint256 sumWeights = 0;
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;
            _targetWeight[token] = newWeights[it];
            sumWeights += newWeights[it];
        }
        uint256 new_max_unit_inflow = sumWeights << 64;
        // We don't want to spend gas on each update, updating the security limit. As a result, we decrease security
        // for lower gas cost.
        if (new_max_unit_inflow >= _max_unit_inflow) {
            // The governance can technically remove the security limit by setting targetTime = max(uint256)
            // while newWeights = (max(uint256)/NUMWEIGHTS) >> 64. Since the router and the governance are
            // to be independent, this is not a security issue.
            // Alt: Call with higher weights and then immediately call with lower weights.
            _max_unit_inflow = new_max_unit_inflow;
            _target_max_unit_inflow = 0;
        } else {
            // Decreases of the security limit can also be a way to remove the security limit. However, if the
            // weights are kept low (=1) (for cheaper tx costs), then this is non-issue since it cannot be
            // decerased further.
            // _max_unit_inflow = current_unit_inflow
            _target_max_unit_inflow = new_max_unit_inflow;
        }
    }

    /**
     * @notice If the governance requests a weight change, this function will adjust the pool weights.
     */
    function _W() internal {
        uint256 adjTarget = _adjustmentTarget;

        if (adjTarget != 0) {
            uint256 currTime = block.timestamp;
            uint256 lastModification = _lastModificationTime;
            if (currTime == lastModification) return; // If no time has passed since last update, then we don't need to update anything.

            // If the current time is past the adjustment, then we need to finalise the weights.
            if (currTime >= adjTarget) {
                for (uint256 it = 0; it < NUMASSETS; it++) {
                    address token = _tokenIndexing[it];
                    if (token == address(0)) break;
                    uint256 targetWeight = _targetWeight[token];
                    // Only save weights if they are differnet.
                    if (_weight[token] != targetWeight) {
                        _weight[token] = targetWeight;
                    }
                }
                // Set weightAdjustmentTime to 0. This ensures the if statement is never entered.
                _adjustmentTarget = 0;
                _lastModificationTime = block.timestamp;
                uint256 target_max_unit_inflow = _target_max_unit_inflow;
                if (target_max_unit_inflow != 0) {
                    _max_unit_inflow = target_max_unit_inflow;
                    _target_max_unit_inflow = 0;
                }
                return;
            }
            for (uint256 it = 0; it < NUMASSETS; it++) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 targetWeight = _targetWeight[token];
                uint256 currentWeight = _weight[token];
                if (currentWeight == targetWeight) {
                    continue;
                }
                uint256 newWeight;
                if (targetWeight > currentWeight) {
                    newWeight =
                        currentWeight +
                        ((targetWeight - currentWeight) *
                            (block.timestamp - lastModification)) /
                        (adjTarget - lastModification);
                } else {
                    newWeight =
                        currentWeight -
                        ((currentWeight - targetWeight) *
                            (block.timestamp - lastModification)) /
                        (adjTarget - lastModification);
                }
                _weight[token] = newWeight;
            }
            _lastModificationTime = block.timestamp;
        }
    }

    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{A}^{A+in} W/(w · ln(2)) dw
     *     = W ln(A_2/A_1)
     * The value is returned as units, which is always in X64.
     * @dev All input amounts should be the raw numbers and not X64.
     * Since units are always denominated in X64, the function
     * should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current pool balance.
     * @param W The pool weight of the token.
     * @return Group specific units in X64 (units are **always** X64).
     */
    function compute_integral(
        uint256 input,
        uint256 A,
        uint256 W
    ) internal pure returns (uint256) {
        // Notice, A + in and A are not X64 but bigdiv64 is used anyway.
        // That is because _log2X64 requires an X64 number.
        // Shifting A + in and A before dividing returns:
        // ((A + in) * 2**64) / ((A) * 2**64) * 2**64 = (A + _in) / A * 2**64
        // Thus the shifting cancels and is not needed.
        return W * log2X64(bigdiv64(A + input, A));
    }

    /**
     * @notice Computes the integral \int_{A}^{A+in} W/(w · ln(2)) dw by lower approximation.
     * After swapping, we know the price to be W/((_A + in) · ln(2)). The amount provided
     * to the seller is then (W · in)/((_A + in) · ln(2)).
     *
     * The value is returned as units, which is always in X64.
     * @dev This function should sometimes be used over _compute_integral
     * if in/_A <= 0.1% since it is cheaper and mathematically simpler.
     * All input amounts should be the raw numbers and not X64.
     * Since units are always denominated in X64, the function
     * should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current pool balance of the in token.
     * @param W The pool weight of the in token.
     * @return Group specific units in X64 (units are **always** X64).
     */
    function approx_compute_integral(
        uint256 input,
        uint256 A,
        uint256 W
    ) internal pure returns (uint256) {
        return bigdiv64((W * input) << 64, ((A + input) * LN2));
    }

    /**
     * @notice Solves the equation U = \int_{A-_out}^{A} W/(w · ln(2)) dw for _out
     *     = B_1 · (1 - 2^(-U/W_0))
     *
     * The value is returned as output token.
     * @dev All input amounts should be the raw numbers and not X64.
     * Since units are always denominated in X64, the function
     * should be treated as mathematically *native*.
     * @param U Input units. Is technically X64 but can be treated as not.
     * @param B The current pool balance of the _out token.
     * @param W The pool weight of the _out token.
     * @return Output denominated in output token.
     */
    function solve_integral(
        uint256 U,
        uint256 B,
        uint256 W
    ) internal pure returns (uint256) {
        return (B * (ONE - invp2X64(U / W))) >> 64;
    }

    /**
     * @notice Solves the equation U = \int_{B-_out}^{B} W/(w · ln(2)) dw for _out
     * by upper approximation. After swapping, we know the price to be W/((B - out) · ln(2)).
     * The amount provided to the seller is then (W · _out)/((A - _out) · ln(2)).
     *     => _out = (B · U · LN2)/(W + U · LN2)
     * @dev This function should sometimes be used over _solve_integral.
     * Since the error is relative to _out and _B, it is difficult
     * to provide relative bounds when the approximation is better
     * than the true equation.
     * All input amounts should be the raw numbers and not X64.
     * Since units are always denominated in X64, the function
     * should be treated as mathematically *native*.
     * @param U Input units. Is technically X64 but can be treated as not.
     * @param B The current pool balance of the _out token.
     * @param W The pool weight of the _out token.
     * @return Output denominated in output token.
     */
    function approx_solve_integral(
        uint256 U,
        uint256 B,
        uint256 W
    ) public pure returns (uint256) {
        uint256 UnitTimesLN2 = mulX64(U, LN2);
        return (B * UnitTimesLN2) / ((W << 64) + UnitTimesLN2);
    }

    /**
     * @notice Solves the equation
     *     \int_{A}^{A + _in} W_A/(w · ln(2)) dw = \int_{B-out}^{B} W_B/(w · ln(2)) dw for out
     * => out = B · (1 - ((A+in)/A)^(-W_A/W_B))
     *
     * Alternatively, the integral can be computed through:
     *      solve_integral(_compute_integral(input, A, W_A), B, W_B).
     *      However, complete_integral is very slightly cheaper since it delays a division.
     *      (Apart from that, the mathematical operations are the same.)
     * @dev All input amounts should be the raw numbers and not X64.
     * @param input The input amount.
     * @param A The current pool balance of the _in token.
     * @param B The current pool balance of the _out token.
     * @param W_A The pool weight of the _in token.
     * @param W_B The pool weight of the _out token.
     * @return Output denominated in output token.
     */
    function complete_integral(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B
    ) internal pure returns (uint256) {
        // (A+input)/A >= 1 as in >= 0. As a result, invfpow should be used.
        // Notice, bigdiv64 is used twice on value not x64. This is because a x64
        // shifted valued is required for invfpow in both arguments.
        uint256 U = ONE -
            invfpowX64(bigdiv64(A + input, A), bigdiv64(W_A, W_B));
        return (B * U) >> 64;
    }

    /**
     * @notice Solves the equation
     *     \int_{A}^{A + _in} W_A/(w · ln(2)) dw = \int_{B-out}^{B} W_B/(w · ln(2)) dw for out
     * by approximation. For the mathematical explanation, see approx_solve_integral
     * and approx_compute_integral.
     *     => out = (B · W_A · in)/(W_B · A + (W_A + W_B) · in)
     *
     * Alternatively, the integral can be computed through:
     *     approx_solve_integral(approx_compute_integral(input, A, W_A), B, W_B).
     * However, this approximation never uses X64 numbers which makes it slightly cheaper.
     * @dev This function never use any X64 mathematics.
     * @param input The input amount.
     * @param A The current pool balance of the _in token.
     * @param B The current pool balance of the _out token.
     * @param W_A The pool weight of the _in token.
     * @param W_B The pool weight of the _out token.
     * @return Output denominated in output token.
     */
    function approx_integral(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B
    ) internal pure returns (uint256) {
        return (B * W_A * input) / (W_B * A + (W_A + W_B) * input);
    }

    /**
     * @notice Computes the return of a SwapToUnits, without executing one.
     * @dev Before letting a user swap, it can be beneficial to viewing the
     * return approximated and not approximated. Then choose the one with
     * the lowest cost (max return & min gas cost).
     * @param from The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @param approx True if SwapToUnits should be approximated (read: approx_compute_integral)
     * @return uint256 specific units in X64 (units are **always** X64).
     */
    function dry_swap_to_unit(
        address from,
        uint256 amount,
        bool approx
    ) public view returns (uint256) {
        uint256 A = IERC20(from).balanceOf(address(this));
        uint256 W = _weight[from];

        if (approx) return approx_compute_integral(amount, A, W);

        return compute_integral(amount, A, W);
    }

    /**
     * @notice Computes the output of a SwapFromUnits, without executing one.
     * @dev Before letting a user swap, it can be beneficial to viewing the
     * return approximated and not approximated. Then choose the one with
     * the lowest cost (max return & min gas cost).
     * @param to The address of the token to buy.
     * @param U The number of units used to buy to.
     * @param approx True if SwapToUnits should be approximated (read: _approx_solve_integral)
     * @return Output denominated in output token.
     */
    function dry_swap_from_unit(
        address to,
        uint256 U,
        bool approx
    ) public view returns (uint256) {
        uint256 B = IERC20(to).balanceOf(address(this)) - _escrowedTokens[to];
        uint256 W = _weight[to];

        if (approx) return approx_solve_integral(U, B, W);

        return solve_integral(U, B, W);
    }

    /**
     * @notice Computes the return of a SwapToAndFromUnits, without executing one.
     * @dev If the pool weights of the 2 tokens are equal, a very simple curve
     * is used and argument approx is ignored.
     * Before letting a user swap, it can be beneficial to viewing the approximated and not approximated.
     * Then choose the one with the lowest cost (max return & min gas cost).
     *
     * If the pool weights of the 2 tokens are equal, a very simple curve
     * is used and argument approx is ignored..
     * @param from The address of the token to sell.
     * @param to The address of the token to buy.
     * @param input The amount of _from token to sell for to token.
     * @param approx True if SwapToUnits should be approximated (read: approx_compute_integral) Is ignored if the tokens weights are equal.
     * @return Output denominated in to token.
     */
    function dry_swap_both(
        address from,
        address to,
        uint256 input,
        bool approx
    ) public view returns (uint256) {
        uint256 A = IERC20(from).balanceOf(address(this));
        uint256 B = IERC20(to).balanceOf(address(this)) - _escrowedTokens[to];
        uint256 W_A = _weight[from];
        uint256 W_B = _weight[to];

        // The swap equation simplifies to the ordinary constant product if the
        // token weights are equal. This equation is even simpler than approx.
        if (W_A == W_B)
            // Saves ~7500 gas.
            return (B * input) / (A + input);

        if (approx) return approx_integral(input, A, B, W_A, W_B);

        return complete_integral(input, A, B, W_A, W_B);
    }

    // /// @notice
    // ///     Returns the number of tokens needed to mint a certain number of pool tokens.
    // ///
    // ///     Solves \int_{A_{0}}^{A_{t}}P \ \left(w\right)d w = \int_{A_{0} +pt}^{A_{t} +tk}P \ \left( w\right)d w
    // ///     for tk.
    // /// @param asset The token address used as underlying for deposit.
    // /// @param poolTokens The number of asset specific pool tokens to mint.
    // /// @return Number of tokens required to mint poolTokens.
    // function liquidity_equation_tk(address asset, uint256 poolTokens)
    //     internal
    //     view
    //     returns (uint256)
    // {
    //     uint256 At = IERC20(asset).balanceOf(address(this));
    //     uint256 A0 = _balance0[asset];
    //     if (At == A0) return poolTokens;

    //     return (At * poolTokens) / A0;
    // }

    // todo: @nonreentrant('lock')
    /**
     * @notice Deposits a symmetrical number of tokens such that poolTokens are minted.
     * This doesn't change the pool price.
     * @dev Requires approvals for all tokens within the pool.
     * @param poolTokens The number of pool tokens to mint.
     */
    function depositAll(uint256 poolTokens) external {
        // Cache totalSupply. This saves up to ~200 gas.
        uint256 initial_totalSupply = totalSupply(); // Not! + _escrowedPoolTokens, since a smaller number results in fewer pool tokens.

        // For later event logging, the amounts transferred to the pool are stored.
        uint256[] memory amounts = new uint256[](NUMASSETS);
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            // Deposits should returns less, so the escrowed tokens are not subtracted.
            uint256 At = IERC20(token).balanceOf(address(this));

            // Number of tokens which can be released given balance0Amount pool tokens.
            uint256 tokenAmount = (At * poolTokens) / initial_totalSupply;

            // Transfer the appropriate number of pool tokens from the user
            // to the pool. (And store for event logging)
            amounts[it] = tokenAmount;
            IERC20(token).safeTransferFrom(
                msg.sender,
                address(this),
                tokenAmount
            ); // dev: User doesn't have enough tokens;
        }

        // Mint the desired number of pool tokens to the user.
        _mint(msg.sender, poolTokens);

        // Emit the event
        emit Deposit(msg.sender, poolTokens, amounts);
    }

    // @nonreentrant('lock')
    /**
     * @notice Burns poolTokens and releases the symmetrical share
     * of tokens to the burner. This doesn't change the pool price.
     * @param poolTokens The number of pool tokens to burn.
     */
    function withdrawAll(uint256 poolTokens, uint256[] calldata minOut) external {
        // cache totalSupply. This saves up to ~200 gas.
        uint256 initial_totalSupply = totalSupply() + _escrowedPoolTokens;

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, poolTokens);

        // For later event logging, the amounts transferred to the pool are stored.
        uint256[] memory amounts = new uint256[](NUMASSETS);
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            // Withdrawals should returns less, so the escrowed tokens are subtracted.
            uint256 At = IERC20(token).balanceOf(address(this)) -  _escrowedTokens[token];

            // Number of tokens which can be released given balance0Amount pool tokens.
            uint256 tokenAmount = (At * poolTokens) / initial_totalSupply;

            require(tokenAmount >= minOut[it], SWAP_RETURN_INSUFFICIENT);
            // Transferring of the released tokens.
            amounts[it] = tokenAmount;
            IERC20(token).safeTransfer(msg.sender, tokenAmount);
        }

        // Emit the event
        emit Withdraw(msg.sender, poolTokens, amounts);
    }

    // @nonreentrant('lock')
    /**
     * @notice A swap between 2 assets which both are inside the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of _toAsset the user wants.
     * @param approx If true, uses (worse) but simpler swapping which can improve swap return and gas costs. If assets weights are equal, this is ignored.
     */
    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut,
        bool approx
    ) public returns (uint256) {
        _W();
        uint256 fee = mulX64(amount, _poolFeeX64);

        // Calculate the swap return value.
        uint256 out = dry_swap_both(fromAsset, toAsset, amount - fee, approx);

        // Check if the calculated returned value is more than the minimum output.
        require(out >= minOut, SWAP_RETURN_INSUFFICIENT);

        // Swap tokens with the user.
        IERC20(toAsset).safeTransfer(msg.sender, out);
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        emit LocalSwap(msg.sender, fromAsset, toAsset, amount, out, fee);

        return out;
    }

    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) public returns (uint256) {
        return localswap(fromAsset, toAsset, amount, minOut, false);
    }

    // @nonreentrant('lock')
    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another pool.
     * @param chain The target chain. Will be converted by the interface to channelId.
     * @param targetPool The target pool on the target chain encoded in bytes32. For EVM chains this can be computed as:
     * Vyper: convert(_poolAddress, bytes32)
     * Solidity: abi.encode(_poolAddress)
     * Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * @param targetUser The recipient of the transaction on _chain. Encoded in bytes32. For EVM chains it can be derived similarly to targetPool.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param amount The number of _fromAsset to sell to the pool.
     * @param minOut The minimum number of returned tokens to the targetUser on the target chain.
     * @param approx Should SwapFromUnits be computed using approximation?
     * @param fallbackUser If the transaction fails send the escrowed funds to this address
     * @param calldata_ Data field if a call should be made on the target chain. Should be encoded abi.encode(address, data)
     * @dev Use the appropriate dry swaps to decide if approximation makes sense.
     * These are the same functions as used by the swap functions, so they will
     * accurately predict the gas cost and swap return.
     */
    function swapToUnits(
        uint32 chain,
        bytes32 targetPool,
        bytes32 targetUser,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        uint8 approx,
        address fallbackUser,
        bytes memory calldata_
    ) public returns (uint256) {
        require(fallbackUser != address(0));
        _W();
        // uint256 fee = mulX64(amount, _poolFeeX64);

        // Calculate the group specific units bought.
        uint256 U = dry_swap_to_unit(
            fromAsset,
            amount - mulX64(amount, _poolFeeX64),
            (approx & 1) > 0
        );

        bytes32 messageHash;

        {
            TokenEscrow memory escrowInformation = TokenEscrow({
                amount: amount - mulX64(amount, _poolFeeX64),
                token: fromAsset
            });

            // Send the purchased units to _targetPool on _chain.
            messageHash = CatalystIBCInterface(_chaininterface)
                .crossChainSwap(
                    chain,
                    targetPool,
                    targetUser,
                    toAssetIndex,
                    U,
                    minOut,
                    (approx & 2) > 0,
                    escrowInformation,
                    calldata_
                );
        }
        // Collect the tokens from the user.
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // Escrow the tokens
        require(_escrowedFor[messageHash] == address(0)); // User cannot supply fallbackUser = address(0)
        _escrowedTokens[fromAsset] += amount - mulX64(amount, _poolFeeX64);
        _escrowedFor[messageHash] = fallbackUser;

        {
            // Governance Fee
            uint256 governanceFee = _governanceFee;
            if (governanceFee != 0) {
                uint256 governancePart = mulX64(
                    mulX64(amount, _poolFeeX64),
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
    
    function swapToUnits(
        uint32 chain,
        bytes32 targetPool,
        bytes32 targetUser,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        uint8 approx,
        address fallbackUser
    ) external returns (uint256) {
        bytes memory calldata_ = new bytes(0);
        return swapToUnits(chain, targetPool, targetUser, fromAsset, toAssetIndex, amount, minOut, approx, fallbackUser, calldata_);
    }

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token (toAsset)
     *  Called exclusively by the chaininterface.
     * @dev Can only be called by the chaininterface, as there is no way to check the validity of units.
     * @param toAssetIndex Index of the asset to be purchased with _U units.
     * @param who The recipient of toAsset
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     * @param approx If the swap approximation should be used over the "true" swap.
     */
    function swapFromUnits(
        uint256 toAssetIndex,
        address who,
        uint256 U,
        uint256 minOut,
        bool approx,
        bytes32 messageHash
    ) public returns (uint256) {
        _W();
        // The chaininterface is the only valid caller of this function, as there cannot
        // be a check of _U. (It is purely a number)
        require(msg.sender == _chaininterface);
        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];

        // Check if the swap is according to the swap limits
        checkAndSetUnitCapacity(U);

        // Calculate the swap return value.
        uint256 purchasedTokens = dry_swap_from_unit(toAsset, U, approx);

        require(minOut <= purchasedTokens, SWAP_RETURN_INSUFFICIENT);

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
        bool approx,
        bytes32 messageHash,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256) {
        uint256 purchasedTokens = swapFromUnits(
            toAssetIndex,
            who,
            U,
            minOut,
            approx,
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



    //@nonreentrant('lock')
    /**
     * @notice Initiate a cross-chain liquidity swap by lowering liquidity
     * and transfer the liquidity units to another pool.
     * @param chain The target chain. Will be converted by the interface to channelId.
     * @param targetPool The target pool on the target chain encoded in bytes32. For EVM chains this can be computed as:
     * Vyper: convert(_poolAddress, bytes32)
     * Solidity: abi.encode(_poolAddress)
     * Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * @param targetUser The recipient of the transaction on _chain. Encoded in bytes32. For EVM chains it can be found similarly to _targetPool.
     * @param poolTokens The number of pool tokens to liquidity Swap
     */
    function outLiquidity(
        uint256 chain,
        bytes32 targetPool,
        bytes32 targetUser,
        uint256 poolTokens,
        uint256 minOut,
        uint8 approx,
        address fallbackUser
    ) external returns (uint256) {
        require(fallbackUser != address(0));
        _W();
        // Cache totalSupply. This saves up to ~200 gas.
        uint256 initial_totalSupply = totalSupply() + _escrowedPoolTokens;

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, poolTokens);
        uint256 WSUM = 0; // Is not X64.
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            WSUM += _weight[token]; // Is not X64.
        }

        uint256 U = log2X64(bigdiv64(initial_totalSupply, initial_totalSupply - poolTokens)) * WSUM;

        bytes32 messageHash;
        {
            LiquidityEscrow memory escrowInformation = LiquidityEscrow({
                poolTokens: poolTokens
            });

            // 2 continued: transfer to target pool.
            messageHash = CatalystIBCInterface(_chaininterface)
                .liquiditySwap(
                    chain,
                    targetPool,
                    targetUser,
                    U,
                    minOut,
                    (approx & 2) > 0,
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
            0,
            messageHash
        );

        return U;
    }


    /**
     * @notice Solves the non-unique swap integral.
     * @dev All input amounts should be the raw numbers and not X64.
     * Since units are always denominated in X64, the function
     * should be treated as mathematically *native*.
     * The function is only to be used for liquidity swaps.
     * @param U Input units. Is technically X64 but can be treated as not.
     * @param W The pool weight of the _out token.
     * @return Output denominated in percentage of pool.
     */
    function arbitrary_solve_integralX64(
        uint256 U,
        uint256 W
    ) internal pure returns (uint256) {
        return (ONE - invp2X64(U / W));
    }

    /**
     * @notice Solves the non-unique swap integral by approximation.
     * @dev read @arbitrary_solve_integral.
     * The function is only to be used for liquidity swaps.
     * @param U Input units. Is technically X64 but can be treated as not.
     * @param W The pool weight of the _out token.
     * @return Output denominated in percentage of pool.
     */
    function arbitrary_approx_solve_integralX64(
        uint256 U,
        uint256 W
    ) public pure returns (uint256) {
        uint256 UnitTimesLN2to64 = U * LN2;
        return (UnitTimesLN2to64) / ((W << 64) + UnitTimesLN2to64 >> 64);
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
        bool approx,
        bytes32 messageHash
    ) external returns (uint256) {
        _W();
        // The chaininterface is the only valid caller of this function, as there cannot
        // be a check of _U. (It is purely a number)
        require(msg.sender == _chaininterface);

        // Check if the swap is according to the swap limits
        checkAndSetUnitCapacity(U);

        // We need to use the incoming units to purchace exactly the pool distribution.
        // The pool contains _numberOfTokensInPool pool tokens, the incoming units needs
        // to be distributed according to the pool weights. 
        address token0;
        uint256 WSUM = 0; // Is not X64.
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (it == 0) token0 = token;
            if (token == address(0)) break;

            WSUM += _weight[token]; // Is not X64.
        }

        // Since we know the relationship between the current pool assets (it being the current balances)
        // we only need to derive one. For simplifity, the first asset is always used.
        // address token0;
        // We could use dry_swap_to_unit(from, amount, approx), but it repeats a bunch of logic.
        // Instead, lets implement it here.
        uint256 poolPercentage;
        {
            // 3. Convert units to an even mix of tokens.
            if (approx) {
                poolPercentage = arbitrary_approx_solve_integralX64(U, WSUM); 
            } else {
                poolPercentage = arbitrary_solve_integralX64(U, WSUM);
            }

        }

        // 4. Deposit the even mix of tokens.
        uint256 ts = totalSupply(); 
        uint256 poolTokens = (poolPercentage*ts) >> 64;
        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);

        // Mint pool tokens for who
        _mint(who, poolTokens);

        emit SwapFromLiquidityUnits(who, U, poolTokens, messageHash);

        return poolTokens;
    }
}
