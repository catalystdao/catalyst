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
 * pricing curve: W/w where W is an asset specific weight and w
 * is the pool balance.
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
contract CatalystSwapPool is CatalystSwapPoolCommon, ReentrancyGuard {
    using SafeERC20 for IERC20;

    //--- ERRORS ---//
    // Errors are defined in interfaces/ICatalystV1PoolErrors.sol


    //--- Config ---//
    // Minimum time parameter adjustments can be made with.
    uint256 constant MIN_ADJUSTMENT_TIME = 60 * 60 * 24 * 7;

    // For other config options, see SwapPoolCommon.sol


    //-- Variables --//
    // There are no variables specific to the volatile pool. See SwapPoolCommon.sol

    /**
     * @notice Configures an empty pool.
     * @dev The @param amp is only used as a sanity check and needs to be set to 10**18 (WAD).
     * If less than NUMASSETS are used to setup the pool
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
     * @param amp Amplification factor. Set to 10**18 for this pool
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
        require(amp == FixedPointMathLib.WAD);  // dev: amplification not set correctly.
        // Check for a misunderstanding regarding how many assets this pool supports.
        require(init_assets.length > 0 && init_assets.length <= NUMASSETS);  // dev: invalid asset count
        
        // Compute the security limit.
        {  //  Stack limitations.
            uint256[] memory initialBalances = new uint256[](NUMASSETS);
            uint256 max_unit_inflow = 0;
            for (uint256 it = 0; it < init_assets.length; it++) {
                address tokenAddress = init_assets[it];
                _tokenIndexing[it] = tokenAddress;
                _weight[tokenAddress] = weights[it];
                require(weights[it] > 0);       // dev: invalid 0-valued weight provided
                // The contract expect tokens to have been sent to it before setup is
                // called. Make sure the pool has more than 0 tokens.
                {
                    uint256 balanceOfSelf = IERC20(tokenAddress).balanceOf(
                        address(this)
                    );
                    initialBalances[it] = balanceOfSelf;
                    require(balanceOfSelf > 0); // dev: 0 tokens provided in setup.
                }

                max_unit_inflow += weights[it];
            }
            
            emit Deposit(depositor, MINTAMOUNT, initialBalances);
            
            // The maximum unit flow is \sum Weights * ln(2). The value is multiplied by WAD 
            // since units are always WAD denominated (note WAD is already included in the LN2 factor).
            _max_unit_inflow = max_unit_inflow * FixedPointMathLib.LN2;
        }

        // Mint pool tokens
        _mint(depositor, MINTAMOUNT);
    }

    /**
     * @notice Allows Governance to modify the pool weights to optimise liquidity.
     * @dev targetTime needs to be more than MIN_ADJUSTMENT_TIME in the future.
     * !Can be abused by governance to disable the security limit!
     * @param targetTime Once reached, _weight[...] = newWeights[...]
     * @param newWeights The new weights to apply
     */
    function modifyWeights(uint256 targetTime, uint256[] calldata newWeights)
        external
        onlyFactoryOwner
    {
        require(targetTime >= block.timestamp + MIN_ADJUSTMENT_TIME); // dev: targetTime must be more than MIN_ADJUSTMENT_TIME in the future.
        
        // Store adjustment information
        _adjustmentTarget = targetTime;
        _lastModificationTime = block.timestamp;

        // Compute sum weight for security limit.
        uint256 sumWeights = 0;
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;
            _targetWeight[token] = newWeights[it];
            sumWeights += newWeights[it];
        }
        uint256 new_max_unit_inflow = sumWeights * FixedPointMathLib.WAD;
        // We don't want to spend gas to update the security limit on each swap. 
        // As a result, we "disable" it by immediately increasing it.
        if (new_max_unit_inflow >= _max_unit_inflow) {
            // immediately set higher security limit to ensure the limit isn't reached.

            // ! The governance can disable the security limit by setting targetTime = max(uint256)
            // ! with newWeights = (max(uint256)/sumWeights) / FixedPointMathLib.WAD. 
            // ! Since the router and the governance are assumbed to be independent, this is not a security issue.
            // ! Alt: Call with higher weights and then immediately call with lower weights.
            _max_unit_inflow = new_max_unit_inflow;
            _target_max_unit_inflow = 0;
        } else {
            // Don't update the security limit now but delay it until the weights are set.

            // Decreases of the security limit can also be a way to remove the security limit. However, if the
            // weights are kept low (=1) (for cheaper tx costs), then this is non-issue since it cannot be
            // decerased further.
            _target_max_unit_inflow = new_max_unit_inflow;
        }

        emit ModifyWeights(targetTime, newWeights);
    }

    /**
     * @notice If the governance requests a weight change, this function will adjust the pool weights.
     * @dev Called first thing on every function depending on weights.
     */
    function _W() internal {
        // We might use adjustment target more than once. Since we don't change it, lets store it.
        uint256 adjTarget = _adjustmentTarget;

        if (adjTarget != 0) {
            // We need to use lastModification again. Store it.
            uint256 lastModification = _lastModificationTime;
            _lastModificationTime = block.timestamp;

            // If no time has passed since last update, then we don't need to update anything.
            if (block.timestamp == lastModification) return; 

            // If the current time is past the adjustment the weights needs to be finalized.
            if (block.timestamp >= adjTarget) {
                for (uint256 it = 0; it < NUMASSETS; it++) {
                    address token = _tokenIndexing[it];
                    if (token == address(0)) break;

                    uint256 targetWeight = _targetWeight[token];
                    // Only save weights if they are differnet.
                    if (_weight[token] != targetWeight) {
                        _weight[token] = targetWeight;
                    }
                }
                // Set weightAdjustmentTime to 0. This ensures the first if statement is never entered.
                _adjustmentTarget = 0;

                uint256 target_max_unit_inflow = _target_max_unit_inflow;
                // If the security limit was decreased, decrease it now.
                if (target_max_unit_inflow != 0) {
                    _max_unit_inflow = target_max_unit_inflow;
                    _target_max_unit_inflow = 0;
                }
                return;
            }

            // Calculate partial weight change
            for (uint256 it = 0; it < NUMASSETS; it++) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;

                uint256 targetWeight = _targetWeight[token];
                uint256 currentWeight = _weight[token];

                // If the weight has already been reached, skip the mathematics.
                if (currentWeight == targetWeight) {
                    continue;
                }

                if (targetWeight > currentWeight) {
                    // if the weights are increased then targetWeight - currentWeight > 0.
                    // Add the change to the current weight.
                    _weight[token] = currentWeight + (
                        (targetWeight - currentWeight) * (block.timestamp - lastModification)
                    ) / (adjTarget - lastModification);
                } else {
                    // if the weights are decreased then targetWeight - currentWeight < 0.
                    // Subtract the change from the current weights.
                    _weight[token] = currentWeight - (
                        (currentWeight - targetWeight) * (block.timestamp - lastModification)
                    ) / (adjTarget - lastModification);
                }
            }
        }
    }

    //--- Swap integrals ---//

    /**
     * @notice Computes the integral \int_{A}^{A+x} W/w dw = W ln((A+x)/A)
     * The value is returned as units, which is always WAD.
     * @dev All input amounts should be the raw numbers and not WAD.
     * Since units are always multiplifed by WAD, the function
     * should be treated as mathematically *native*.
     * @param input The input amount.
     * @param A The current pool balance of the x token.
     * @param W The weight of the x token.
     * @return uint256 Group specific units (units are **always** WAD).
     */
    function compute_integral(
        uint256 input,
        uint256 A,
        uint256 W
    ) internal pure returns (uint256) {
        // Notice, A + in and A are not WAD but divWadDown is used anyway.
        // That is because lnWad requires a scaled number.
        return W * uint256(FixedPointMathLib.lnWad(int256(FixedPointMathLib.divWadDown(A + input, A))));
    }

    /**
     * @notice Solves the equation U = \int_{B-y}^{B} W/w dw for y = B · (1 - exp(-U/W))
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
        uint256 W
    ) internal pure returns (uint256) {
        return (B * (FixedPointMathLib.WAD - uint256(FixedPointMathLib.expWad(
            -int256(U / W)
        )))) / FixedPointMathLib.WAD;
    }

    /**
     * @notice Solves the equation
     *     \int_{A}^{A+x} W_a/w dw = \int_{B-y}^{B} W_b/w dw for y =  B · (1 - ((A+x)/A)^(-W_a/W_b))
     *
     * Alternatively, the integral can be computed through:
     *      solve_integral(_compute_integral(input, A, W_A), B, W_B).
     *      However, complete_integral is very slightly cheaper since it delays a division.
     *      (Apart from that, the mathematical operations are the same.)
     * @dev All input amounts should be the raw numbers and not X64.
     * @param input The input amount.
     * @param A The current pool balance of the x token.
     * @param B The current pool balance of the y token.
     * @param W_A The weight of the x token.
     * @param W_B TThe weight of the y token.
     * @return uint256 Output denominated in output token.
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
        uint256 U = FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
            int256(FixedPointMathLib.divWadDown(A + input, A)),
            int256(FixedPointMathLib.divWadDown(W_A, W_B))
        ));
        return (B * U) / FixedPointMathLib.WAD;
    }

    /**
     * @notice Solves the non-unique swap integral.
     * @dev The function is only to be used for liquidity swaps.
     * Read solve_integral for more documentation.
     * @param U Input units.
     * @param W The generalised weights.
     * @return uint256 Output denominated in percentage of pool.
     */
    function arbitrary_solve_integral(
        uint256 U,
        uint256 W
    ) internal pure returns (uint256) {
        // Compute the non pool ownership share. (1 - pool ownership share)
        uint256 npos = uint256(FixedPointMathLib.expWad(-int256(U / W)));
        
        // Compute the pool owner share after liquidity has been added.
        // (solve share = pt/(PT+pt) for pt.)
        return FixedPointMathLib.divWadDown(FixedPointMathLib.WAD - npos, npos);
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

        // If a token is not part of the pool, W is 0. This returns 0 by
        // multiplication with 0.
        return compute_integral(amount, A, W);
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
        return solve_integral(U, B, W);
    }

    /**
     * @notice Computes the output of SwapToAndFromUnits.
     * @dev If the pool weights of the 2 tokens are equal, a very simple curve is used.
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

        // The swap equation simplifies to the ordinary constant product if the
        // token weights are equal. This equation is even simpler than approx.
        if (W_A == W_B)
            // Saves ~7500 gas.
            return (B * amount) / (A + amount);

        // If the token doesn't exist, W_A is 0.
        // Then invfpowX64 returns 1 which is subtracted from 1 => returns 0.
        return complete_integral(amount, A, B, W_A, W_B);
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
        // Smaller initial_totalSupply => fewer pool tokens minted: _escrowedPoolTokens is not added.
        uint256 initial_totalSupply = totalSupply(); 

        uint256 WSUM = 0;
        uint256 U = 0;
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            uint256 weight = _weight[token];
            WSUM += weight;

             // A high => less units returned. Do not subtract the escrow amount
            uint256 At = IERC20(token).balanceOf(address(this));

            // Save gas if the user provides no tokens.
            if (tokenAmounts[it] == 0) continue;

            U += compute_integral(tokenAmounts[it], At, weight);

            IERC20(token).safeTransferFrom(
                msg.sender,
                address(this),
                tokenAmounts[it]
            ); // dev: Token withdrawal from user failed.
        }

        // Subtract fee from U. This stops people from using deposit and withdrawal as method of swapping.
        // To reduce costs, there the governance fee is not included. This does result in deposit+withdrawal
        // working as a way to circumvent the governance fee.
        U = FixedPointMathLib.mulWadDown(U, FixedPointMathLib.WAD - _poolFee);

        uint256 poolTokens = (initial_totalSupply * arbitrary_solve_integral(U, WSUM)) / FixedPointMathLib.WAD;

        // Check that the minimum output is honored.
        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);

        // Emit the deposit event
        emit Deposit(msg.sender, poolTokens, tokenAmounts);

        // Mint the desired number of pool tokens to the user.
        _mint(msg.sender, poolTokens);

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
        // Cache totalSupply. This saves up to ~200 gas.
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
            uint256 At = IERC20(token).balanceOf(address(this)) - _escrowedTokens[token];

            // Number of tokens which can be released given poolTokens.
            uint256 tokenAmount = (At * poolTokens) / initial_totalSupply;

            require(tokenAmount >= minOut[it], SWAP_RETURN_INSUFFICIENT);

            // Transferring of the released tokens.
            amounts[it] = tokenAmount;
            IERC20(token).safeTransfer(msg.sender, tokenAmount);
        }

        // Emit the event
        emit Withdraw(msg.sender, poolTokens, amounts);

        return amounts;
    }

    /**
     * @notice Burns poolTokens and release a token distribution which can be set by the user.
     * @dev It is advised that the withdraw matches the pool's %token distribution.
     * @param poolTokens The number of pool tokens to withdraw
     * @param withdrawRatioX64 The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is Wad.
     * @param minOuts The minimum number of tokens withdrawn.
     */
    function withdrawMixed(
        uint256 poolTokens,
        uint256[] calldata withdrawRatioX64,
        uint256[] calldata minOuts
    ) nonReentrant() external returns(uint256[] memory) {
        // cache totalSupply. This saves a bit of gas.
        uint256 initial_totalSupply = totalSupply() + _escrowedPoolTokens;

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, poolTokens);

        address[] memory tokenIndexed = new address[](NUMASSETS);
        uint256[] memory weights = new uint256[](NUMASSETS);

        // Compute the weight sum. And cache all storage.
        uint256 WSUM = 0;
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;
            tokenIndexed[it] = token;
            uint256 weight = _weight[token];
            weights[it] = weight;
            WSUM += weight;
        }

        // Compute the unit worth of the pool tokens.
        uint256 U = uint256(FixedPointMathLib.lnWad(
            int256(FixedPointMathLib.divWadDown(initial_totalSupply, initial_totalSupply - poolTokens)
        ))) * WSUM;

        // For later event logging, the amounts transferred to the pool are stored.
        uint256[] memory amounts = new uint256[](NUMASSETS);
        for (uint256 it = 0; it < NUMASSETS; it++) {
            // If there are no units remaning, stop the loop.
            if (U == 0) break;

            // Find how many units are to be used for used for the token in the current it.
            uint256 U_i = (U * withdrawRatioX64[it]) / FixedPointMathLib.WAD;
            if (U_i == 0) continue;  // If no tokens are to be used, skip the logic.
            U -= U_i;  // Subtract the number of units used.
            address token = tokenIndexed[it];

            // Withdrawals should returns less, so the escrowed tokens are subtracted.
            uint256 At = IERC20(token).balanceOf(address(this)) - _escrowedTokens[token];

            // Units are shared between "liquidity units" and "token units". As such, we just
            // need to convert the units to tokens.
            uint256 tokenAmount = solve_integral(U_i, At, weights[it]);

            require(minOuts[it] <= tokenAmount, SWAP_RETURN_INSUFFICIENT);
            amounts[it] = tokenAmount;

            // Transfer the released tokens.
            IERC20(token).safeTransfer(msg.sender, tokenAmount);
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
     */
    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) nonReentrant() public returns (uint256) {
        _W();
        uint256 fee = FixedPointMathLib.mulWadDown(amount, _poolFee);

        // Calculate the swap return value.
        uint256 out = dry_swap_both(fromAsset, toAsset, amount - fee);

        // Check if the calculated returned value is more than the minimum output.
        require(out >= minOut, SWAP_RETURN_INSUFFICIENT);

        IERC20(toAsset).safeTransfer(msg.sender, out);
        IERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        emit LocalSwap(msg.sender, fromAsset, toAsset, amount, out);

        return out;
    }

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another pool.
     * @dev Encoding addresses in bytes32 can be done be computed with:
     * Vyper: convert(<poolAddress>, bytes32)
     * Solidity: abi.encode(<poolAddress>)
     * Brownie: brownie.convert.to_bytes(<poolAddress>, type_str="bytes32")
     * @param channelId The target chain identifier.
     * @param targetPool The target pool on the target chain encoded in bytes32.
     * @param targetUser The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param amount The number of fromAsset to sell to the pool.
     * @param minOut The minimum number of returned tokens to the targetUser on the target chain.
     * @param fallbackUser If the transaction fails send the escrowed funds to this address
     * @param calldata_ Data field if a call should be made on the target chain. 
     * Should be encoded abi.encode(<address>,<data>)
     */
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
        _W();
        // uint256 fee = mulX64(amount, _poolFee);

        // Calculate the group specific units bought.
        uint256 U = dry_swap_to_unit(
            fromAsset,
            amount - FixedPointMathLib.mulWadDown(amount, _poolFee)
        );

        bytes32 messageHash;
        {
            TokenEscrow memory escrowInformation = TokenEscrow({
                amount: amount - FixedPointMathLib.mulWadDown(amount, _poolFee),
                token: fromAsset
            });

            // Send the purchased units to targetPool on the target chain..
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
        // The chaininterface is the only valid caller of this function.
        require(msg.sender == _chaininterface);
        _W();

        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];

        // Check if the swap is according to the swap limits
        checkAndSetUnitCapacity(U);

        // Calculate the swap return value.
        uint256 purchasedTokens = dry_swap_from_unit(toAsset, U);

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
        // Update weights
        _W();

        uint256 initial_totalSupply = totalSupply() + _escrowedPoolTokens;

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, poolTokens);

        // Compute the weight sum.
        uint256 WSUM = 0;
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            WSUM += _weight[token];
        }

        // Compute the unit value of the provided poolTokens.
        uint256 U = uint256(FixedPointMathLib.lnWad(int256(
            FixedPointMathLib.divWadDown(initial_totalSupply, initial_totalSupply - poolTokens)
        ))) * WSUM;


        bytes32 messageHash;
        {
            
            LiquidityEscrow memory escrowInformation = LiquidityEscrow({
                poolTokens: poolTokens
            });

            // Transfer the units to the target pools.
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
        _W();

        // Check if the swap is according to the swap limits
        checkAndSetUnitCapacity(U);

        // Compute the weight sum.
        address token0;
        uint256 WSUM = 0; // Is not X64.
        for (uint256 it = 0; it < NUMASSETS; it++) {
            address token = _tokenIndexing[it];
            if (it == 0) token0 = token;
            if (token == address(0)) break;

            WSUM += _weight[token]; // Is not X64.
        }

        // Use the arbitarty integral to compute the pool ownership percentage.
        // The function then converts ownership percentage into mint %.
        uint256 poolTokens = (arbitrary_solve_integral(U, WSUM) * totalSupply())/FixedPointMathLib.WAD;

        // Check if the user would accept the mint.
        require(minOut <= poolTokens, SWAP_RETURN_INSUFFICIENT);

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
            // If UF < U and we do UF - U < 0 underflow => bad.
            if (UF > U) {
                _unit_flow = UF - U; // Does not underflow since _unit_flow > U.
            } else if (UF != 0) {
                // If UF == 0, then we shouldn't do anything. Skip that case.
                // when UF <= U => UF - U <= 0 => max(UF - U, 0) = 0
                _unit_flow = 0;
            }
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

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the pool, we should expect that there is exactly
        // the inswapped amount of trust in the pool. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UF = _unit_flow;
            // If UF < U and we do UF - U < 0 underflow => bad.
            if (UF > U) {
                _unit_flow = UF - U; // Does not underflow since _unit_flow > U.
            } else if (UF != 0) {
                // If UF == 0, then we shouldn't do anything. Skip that case.
                // when UF <= U => UF - U <= 0 => max(UF - U, 0) = 0
                _unit_flow = 0;
            }
        }
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
