//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";
import "./FixedPointMathLib.sol";
import "./CatalystIBCInterface.sol";
import "./VaultCommon.sol";
import "./ICatalystV1Vault.sol";

/**
 * @title Catalyst: The Multi-Chain Swap vault
 * @author Catalyst Labs
 * @notice Catalyst multi-chain swap vault using the asset specific
 * pricing curve: W/w where W is an asset-specific weight and w
 * is the vault balance.
 *
 * The following contract supports between 1 and 3 assets for
 * atomic swaps. To increase the number of tokens supported,
 * change MAX_ASSETS to the desired maximum token amount.
 * This constant is set in "SwapVaultCommon.sol"
 *
 * The swapvault implements the ERC20 specification, such that the
 * contract will be its own vault token.
 * @dev This contract is deployed inactive: It cannot be used as a
 * swap vault as is. To use it, a proxy contract duplicating the
 * logic of this contract needs to be deployed. In Vyper, this
 * can be done through (vy >=0.3.4) create_minimal_proxy_to.
 * In Solidity, this can be done through OZ clones: Clones.clone(...)
 * After deployment of the proxy, call setup(...). This will
 * initialize the vault and prepare it for cross-chain transactions.
 *
 * If connected to a supported cross-chain interface, call
 * setConnection to connect the vault with vaults on other chains.
 *
 * Finally, call finishSetup to give up the deployer's control
 * over the vault. 
 * !If finishSetup is not called, the vault can be drained!
 */
contract CatalystVaultVolatile is CatalystVaultCommon {
    using SafeTransferLib for ERC20;

    //--- ERRORS ---//
    // Errors are defined in interfaces/ICatalystV1VaultErrors.sol


    //--- Config ---//
    // Minimum time parameter adjustments can be made over.
    uint256 constant MIN_ADJUSTMENT_TIME = 7 days;

    // For other config options, see SwapVaultCommon.sol

    //-- Variables --//
    mapping(address => uint256) public _targetWeight;

    constructor(address factory_) CatalystVaultCommon(factory_) {}

    /**
     * @notice Configures an empty vault.
     * @dev The @param amp is only used as a sanity check and needs to be set to 10**18 (WAD).
     * If less than MAX_ASSETS are used to initiate the vault
     * let the remaining <assets> be ZERO_ADDRESS / address(0)
     *
     * Unused weights can be whatever. (0 is recommended.)
     *
     * The initial token amounts should have been sent to the vault before setup is called.
     * Since someone can call setup can claim the initial tokens, this needs to be
     * done atomically!
     *
     * If 0 of a token in assets is provided, the setup reverts.
     * @param assets A list of the token addresses associated with the vault
     * @param weights The weights associated with the tokens. 
     * If set to values with low resolution (<= 10*5), this should be viewed as
     * opt out of governance weight adjustment. This is not enforced.
     * @param amp Amplification factor. Set to 10**18 for this vault
     * @param depositor The address depositing the initial token balances.
     */
    function initializeSwapCurves(
        address[] calldata assets,
        uint256[] calldata weights,
        uint256 amp,
        address depositor
    ) public override {
        // May only be invoked by the FACTORY. The factory only invokes this function for proxy contracts.
        require(msg.sender == FACTORY && _tokenIndexing[0] == address(0));  // dev: swap curves may only be initialized once by the factory
        // Check that the amplification is correct.
        require(amp == FixedPointMathLib.WAD);  // dev: amplification not set correctly.
        // Note there is no need to check whether assets.length/weights.length are valid, as invalid arguments
        // will either cause the function to fail (e.g. if assets.length > MAX_ASSETS the assignment
        // to initialBalances[it] will fail) or will cause the vault to get initialized with an undesired state
        // (and the vault shouldn't be used by anyone until its configuration has been finalised). 
        // In any case, the factory does check for valid assets/weights arguments to prevent erroneous configurations. 

        // Compute the security limit.
        uint256[] memory initialBalances = new uint256[](MAX_ASSETS);
        uint256 maxUnitCapacity = 0;
        for (uint256 it; it < assets.length;) {

            address tokenAddress = assets[it];
            _tokenIndexing[it] = tokenAddress;

            uint256 weight = weights[it];
            require(weight != 0);       // dev: invalid 0-valued weight provided
            _weight[tokenAddress] = weight;

            // The contract expects the tokens to have been sent to it before setup is
            // called. Make sure the vault has more than 0 tokens.
            // Reverts if tokenAddress is address(0).
            uint256 balanceOfSelf = ERC20(tokenAddress).balanceOf(address(this));
            require(balanceOfSelf != 0); // dev: 0 tokens provided in setup.
            initialBalances[it] = balanceOfSelf;

            maxUnitCapacity += weight;

            unchecked {
                it++;
            }
        }

        // The maximum unit flow is \sum Weights * ln(2). The value is multiplied by WAD 
        // since units are always WAD denominated (note WAD is already included in the LN2 factor).
        _maxUnitCapacity = maxUnitCapacity * FixedPointMathLib.LN2;

        // Mint vault tokens for vault creator.
        _mint(depositor, INITIAL_MINT_AMOUNT);

        emit Deposit(depositor, INITIAL_MINT_AMOUNT, initialBalances);
    }

    /**
     * @notice Allows Governance to modify the vault weights to optimise liquidity.
     * @dev targetTime needs to be more than MIN_ADJUSTMENT_TIME in the future.
     * It is implied that if the existing weights are low <≈100, then 
     * the governance is not allowed to change vault weights. This is because
     * the update function is not made for step sizes (which the result would be if)
     * trades are frequent and weights are small.
     * Weights must not be set to 0. This allows someone to exploit the localSwap simplification
     * with a token not belonging to the vault. (Set weight to 0, localSwap from token not part of
     * the vault. Since 0 == 0 => use simplified swap curve. Swap goes through.)
     * @param targetTime Once reached, _weight[...] = newWeights[...]
     * @param newWeights The new weights to apply
     */
    function setWeights(uint256 targetTime, uint256[] calldata newWeights) external onlyFactoryOwner {
        unchecked {
            require(targetTime >= block.timestamp + MIN_ADJUSTMENT_TIME); // dev: targetTime must be more than MIN_ADJUSTMENT_TIME in the future.
            require(targetTime <= block.timestamp + 365 days); // dev: Target time cannot be too far into the future.
        }

        // Store adjustment information
        _adjustmentTarget = targetTime;
        _lastModificationTime = block.timestamp;

        // Save the target weights
        for (uint256 it; it < MAX_ASSETS;) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

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
            // Update security limit unit capacity
            _maxUnitCapacity = wsum * FixedPointMathLib.LN2;
        }
    }

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
    function _calcPriceCurveArea(
        uint256 input,
        uint256 A,
        uint256 W
    ) internal pure returns (uint256) {
        // Notice, A + in and A are not WAD but divWadDown is used anyway.
        // That is because lnWad requires a scaled number.
        return W * uint256(FixedPointMathLib.lnWad(int256(FixedPointMathLib.divWadDown(A + input, A))));    // int256 casting is safe. If overflows, it returns negative. lnWad fails on negative numbers. If the vault balance is high, this is unlikely.
    }

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
    function _calcPriceCurveLimit(
        uint256 U,
        uint256 B,
        uint256 W
    ) internal pure returns (uint256) {
        return FixedPointMathLib.mulWadDown(
            B,
            FixedPointMathLib.WAD - uint256(FixedPointMathLib.expWad(-int256(U / W)))   // int256 casting is initially not safe. If overflow, the equation becomes: 1 - exp(U/W) => exp(U/W) > 1. In this case, Solidity's built-in safe math protection catches the overflow.
        );
    }

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
    function _calcCombinedPriceCurves(
        uint256 input,
        uint256 A,
        uint256 B,
        uint256 W_A,
        uint256 W_B
    ) internal pure returns (uint256) {
        // uint256 U = FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(
        //     int256(FixedPointMathLib.divWadDown(A + input, A)),
        //     int256(FixedPointMathLib.divWadDown(W_A, W_B))
        //)); 
        // return (B * U) / FixedPointMathLib.WAD;
        return _calcPriceCurveLimit(_calcPriceCurveArea(input, A, W_A), B, W_B);
    }

    /**
     * @notice Solves the generalised swap integral.
     * @dev Based on _calcPriceCurveLimit but the multiplication by the
     * specific token is never done.
     * @param U Input units.
     * @param W The generalised weights.
     * @return uint256 Output denominated in vault share.
     */
    function _calcPriceCurveLimitShare(
        uint256 U,
        uint256 W
    ) internal pure returns (uint256) {
        // Compute the non vault ownership share. (1 - vault ownership share)
        uint256 npos = uint256(FixedPointMathLib.expWad(-int256(U / W)));   // int256 casting is initially not safe. If overflow, the equation becomes: exp(U/W). In this case, when subtracted from 1 (later), Solidity's built-in safe math protection catches the overflow since exp(U/W) > 1.
        
        // Compute the vault owner share before liquidity has been added.
        // (solve share = pt/(PT+pt) for pt.)
        return FixedPointMathLib.divWadDown(FixedPointMathLib.WAD - npos, npos);
    }

    /**
     * @notice Computes the return of SendAsset.
     * @dev Returns 0 if from is not a token in the vault
     * @param fromAsset The address of the token to sell.
     * @param amount The amount of from token to sell.
     * @return uint256 Group-specific units.
     */
    function calcSendAsset(
        address fromAsset,
        uint256 amount
    ) public view override returns (uint256) {
        // A high => fewer units returned. Do not subtract the escrow amount
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 W = _weight[fromAsset];

        // If a token is not part of the vault, W is 0. This returns 0 by
        // multiplication with 0.
        return _calcPriceCurveArea(amount, A, W);
    }

    /**
     * @notice Computes the output of ReceiveAsset.
     * @dev Reverts if to is not a token in the vault
     * @param toAsset The address of the token to buy.
     * @param U The number of units used to buy to.
     * @return uint256 Number of purchased tokens.
     */
    function calcReceiveAsset(
        address toAsset,
        uint256 U
    ) public view override returns (uint256) {
        // B low => fewer tokens returned. Subtract the escrow amount to decrease the balance.
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - _escrowedTokens[toAsset];
        uint256 W = _weight[toAsset];

        // If someone were to purchase a token which is not part of the vault on setup
        // they would just add value to the vault. We don't care about it.
        // However, it will revert since the solved integral contains U/W and when
        // W = 0 then U/W returns division by 0 error.
        return _calcPriceCurveLimit(U, B, W);
    }

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
        address fromAsset,
        address toAsset,
        uint256 amount
    ) public view override returns (uint256) {
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - _escrowedTokens[toAsset];
        uint256 W_A = _weight[fromAsset];
        uint256 W_B = _weight[toAsset];

        // The swap equation simplifies to the ordinary constant product if the
        // token weights are equal.
        if (W_A == W_B)
            // Saves gas and is exact.
            // NOTE: If W_A == 0 and W_B == 0 => W_A == W_B => The calculation will not fail.
            // This is not a problem, since W_B != 0 for assets contained in the vault, and hence a 0-weighted asset 
            // (i.e. not contained in the vault) cannot be used to extract an asset contained in the vault.
            return (B * amount) / (A + amount);

        // If either token doesn't exist, their weight is 0.
        // Then powWad returns 1 which is subtracted from 1 => returns 0.
        return _calcCombinedPriceCurves(amount, A, B, W_A, W_B);
    }

    /**
     * @notice Deposits a  user-configurable amount of tokens. 
     * @dev The swap fee is imposed on deposits.
     * Requires approvals for all tokens within the vault.
     * It is advised that the deposit matches the vault's %token distribution.
     * Deposit is done by converting tokenAmounts into units and then using
     * the macro for units to vault tokens. (_calcPriceCurveLimitShare)
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of vault tokens to be minted.
     * @return uint256 The number of minted vault tokens.
     */
    function depositMixed(
        uint256[] calldata tokenAmounts,
        uint256 minOut
    ) nonReentrant external override returns(uint256) {
        // Smaller initialTotalSupply => fewer vault tokens minted: _escrowedVaultTokens is not added.
        uint256 initialTotalSupply = totalSupply; 

        uint256 U = 0;
        for (uint256 it; it < MAX_ASSETS;) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            // Save gas if the user provides no tokens.
            if (tokenAmounts[it] == 0) {
                unchecked {
                    it++;
                }
                continue;
            }

             // A high => fewer units returned. Do not subtract the escrow amount
            uint256 At = ERC20(token).balanceOf(address(this));

            U += _calcPriceCurveArea(tokenAmounts[it], At, _weight[token]);

            ERC20(token).safeTransferFrom(
                msg.sender,
                address(this),
                tokenAmounts[it]
            ); // dev: Token withdrawal from user failed.

            unchecked {
                it++;
            }
        }

        // Subtract fee from U. This stops people from using deposit and withdrawal as a method of swapping.
        // To reduce costs, the governance fee is not taken. Swapping through deposit+withdrawal
        // circumvents the governance fee. However, traders are disincentivised by a higher gas cost.
        unchecked {
            // Normally U is lower than the sum of the weights * LN2. This is much lower than 2**256-1
            // And if U overflows, then it becomes smaller.
            U = (U * (FixedPointMathLib.WAD - _vaultFee))/FixedPointMathLib.WAD;
        }

        // Fetch wsum.
        uint256 wsum = _maxUnitCapacity / FixedPointMathLib.LN2;

        // Compute the number of vault tokens minted to the user. Notice that _calcPriceCurveLimitShare > 1 thus more
        // than the totalSupply can be minted given sufficiently large U.
        uint256 vaultTokens = FixedPointMathLib.mulWadDown(initialTotalSupply, _calcPriceCurveLimitShare(U, wsum));

        // Check that the minimum output is honoured.
        if (minOut > vaultTokens) revert ReturnInsufficient(vaultTokens, minOut);

        // Emit the deposit event
        emit Deposit(msg.sender, vaultTokens, tokenAmounts);

        // Mint the desired number of vault tokens to the user.
        _mint(msg.sender, vaultTokens);

        return vaultTokens;
    }

    /**
     * @notice Burns vaultTokens and releases the symmetrical share of tokens to the burner. 
     * This doesn't change the vault price.
     * @dev This is the cheapest way to withdraw.
     * @param vaultTokens The number of vault tokens to burn.
     * @param minOut The minimum token output. If less is returned, the transaction reverts.
     * @return uint256[] memory An array containing the amounts withdrawn.
     */
    function withdrawAll(
        uint256 vaultTokens,
        uint256[] calldata minOut
    ) nonReentrant external override returns(uint256[] memory) {
        // Cache totalSupply. This saves up to ~200 gas.
        uint256 initialTotalSupply = totalSupply + _escrowedVaultTokens;

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, vaultTokens);

        // For later event logging, the amounts transferred from the vault are stored.
        uint256[] memory amounts = new uint256[](MAX_ASSETS);
        for (uint256 it; it < MAX_ASSETS;) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;

            // Withdrawals should return less, so the escrowed tokens are subtracted.
            uint256 At = ERC20(token).balanceOf(address(this)) - _escrowedTokens[token];

            // Number of tokens which can be released given vaultTokens.
            uint256 tokenAmount = (At * vaultTokens) / initialTotalSupply;

            // Check if the user is satisfied with the output.
            if (minOut[it] > tokenAmount)
                revert ReturnInsufficient(tokenAmount, minOut[it]);

            // Store the token amount.
            amounts[it] = tokenAmount;

            // Transfer the released tokens to the user.
            ERC20(token).safeTransfer(msg.sender, tokenAmount);

            unchecked {
                it++;
            }
        }

        // Emit the event
        emit Withdraw(msg.sender, vaultTokens, amounts);

        return amounts;
    }

    /**
     * @notice Burns vaultTokens and release a token distribution which can be set by the user.
     * @dev It is advised that the withdrawal matches the vault's %token distribution.
     * @param vaultTokens The number of vault tokens to withdraw.
     * @param withdrawRatio The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is WAD.
     * @param minOut The minimum number of tokens withdrawn.
     * @return uint256[] memory An array containing the amounts withdrawn.
     */
    function withdrawMixed(
        uint256 vaultTokens,
        uint256[] calldata withdrawRatio,
        uint256[] calldata minOut
    ) nonReentrant external override returns(uint256[] memory) {
        // cache totalSupply. This saves a bit of gas.
        uint256 initialTotalSupply = totalSupply + _escrowedVaultTokens;

        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, vaultTokens);

        // Fetch wsum.
        uint256 wsum = _maxUnitCapacity / FixedPointMathLib.LN2;

        // Compute the unit worth of the vault tokens.
        uint256 U = uint256(FixedPointMathLib.lnWad( // uint256: ln computed of a value always > 1, hence always positive
            int256(FixedPointMathLib.divWadDown(initialTotalSupply, initialTotalSupply - vaultTokens)    // int265: if vaultTokens is almost equal to initialTotalSupply this can overflow the cast. The result is a negative input to lnWad which fails.
        ))) * wsum;

        // For later event logging, the amounts transferred to the vault are stored.
        uint256[] memory amounts = new uint256[](MAX_ASSETS);
        for (uint256 it; it < MAX_ASSETS;) {
            address token = _tokenIndexing[it]; // Collect token from memory
            if (token == address(0)) break;

            // Units allocated for the specific token.
            uint256 U_i = FixedPointMathLib.mulWadDown(U, withdrawRatio[it]);
            if (U_i == 0) {
                // There should not be a non-zero withdrawRatio after a withdraw ratio of 1
                if (withdrawRatio[it] != 0) revert WithdrawRatioNotZero();
                if (minOut[it] != 0) revert ReturnInsufficient(0, minOut[it]);
                
                unchecked {
                    it++;
                }
                continue;
            }
            U -= U_i;  // Subtract the number of units used. This will underflow for malicious withdrawRatios > 1.

            // Withdrawals should returns less, so the escrowed tokens are subtracted.
            uint256 At = ERC20(token).balanceOf(address(this)) - _escrowedTokens[token];

            // Units are shared between "liquidity units" and "token units". As such, we just
            // need to convert the units to tokens.
            uint256 tokenAmount = _calcPriceCurveLimit(U_i, At, _weight[token]);

            // Ensure the output satisfies the user.
            if (minOut[it] > tokenAmount)
                revert ReturnInsufficient(tokenAmount, minOut[it]);

            // Store amount for withdraw event
            amounts[it] = tokenAmount;

            // Transfer the released tokens to the user.
            ERC20(token).safeTransfer(msg.sender, tokenAmount);

            unchecked {
                it++;
            }
        }
        if (U != 0) revert UnusedUnitsAfterWithdrawal(U);

        // Emit the event
        emit Withdraw(msg.sender, vaultTokens, amounts);

        return amounts;
    }

    /**
     * @notice A swap between 2 assets within the vault. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of toAsset the user wants.
     * @return uint256 The number of tokens purchased.
     */
    function localSwap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) nonReentrant external override returns (uint256) {
        _updateWeights();
        uint256 fee = FixedPointMathLib.mulWadDown(amount, _vaultFee);

        // Calculate the return value.
        uint256 out = calcLocalSwap(fromAsset, toAsset, amount - fee);

        // Check if the calculated returned value is more than the minimum output.
        if (minOut > out) revert ReturnInsufficient(out, minOut);

        // Transfer tokens to the user and collect tokens from the user.
        ERC20(toAsset).safeTransfer(msg.sender, out);
        ERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // Governance Fee
        _collectGovernanceFee(fromAsset, fee);

        emit LocalSwap(msg.sender, fromAsset, toAsset, amount, out);

        return out;
    }

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another vault.
     * @dev To encode addresses in bytes32 the functions below can be used:
     * Vyper: convert(<vaultAddress>, bytes32)
     * Solidity: abi.encode(<vaultAddress>)
     * Brownie: brownie.convert.to_bytes(<vaultAddress>, type_str="bytes32")
     * @param channelId The target chain identifier.
     * @param toVault The target vault on the target chain encoded in bytes32.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target vault.
     * @param amount The number of fromAsset to sell to the vault.
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with evm being: abi.encode(bytes20(<address>), <data>)
     * @return uint256 The number of units minted.
     */
    function sendAsset(
        bytes32 channelId,
        bytes memory toVault,
        bytes memory toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        bytes memory calldata_
    ) nonReentrant public override returns (uint256) {
        // Only allow connected vaults
        if (!_vaultConnection[channelId][toVault]) revert VaultNotConnected(channelId, toVault);
        require(fallbackUser != address(0));
        require(toVault.length == 65);  // dev: Vault addresses are uint8 + 64 bytes.
        require(toAccount.length == 65);  // dev: Account addresses are uint8 + 64 bytes.

        _updateWeights();

        uint256 fee = FixedPointMathLib.mulWadDown(amount, _vaultFee);

        // Calculate the group-specific units bought.
        uint256 U = calcSendAsset(fromAsset, amount - fee);

        // Send the purchased units to toVault on the target chain.
        CatalystIBCInterface(_chainInterface).sendCrossChainAsset(
            channelId,
            toVault,
            toAccount,
            toAssetIndex,
            U,
            minOut,
            amount - fee,
            fromAsset,
            calldata_
        );

        // Only need to hash info that is required by the escrow (+ some extra for randomisation)
        // No need to hash context (as token/liquidity escrow data is different), fromVault, toVault, targetAssetIndex, minOut, CallData
        bytes32 sendAssetHash = _computeSendAssetHash(
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            amount - fee,   // Required! to validate release escrow data
            fromAsset,      // Required! to validate release escrow data
            uint32(block.number) // May overflow, but this is desired (% 2**32)
        );

        // Escrow the tokens used to purchase units. These will be sent back if transaction
        // doesn't arrive / timeout.
        require(_escrowedTokensFor[sendAssetHash] == address(0)); // dev: Escrow already exists.
        _escrowedTokensFor[sendAssetHash] = fallbackUser;
        unchecked {
            _escrowedTokens[fromAsset] += amount - fee;
        }

        // Collect the tokens from the user.
        ERC20(fromAsset).safeTransferFrom(msg.sender, address(this), amount);

        // Governance Fee
        _collectGovernanceFee(fromAsset, fee);

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit.

        emit SendAsset(
            channelId,
            toVault,
            toAccount,
            fromAsset,
            toAssetIndex,
            amount,
            minOut,
            U,
            fee
        );

        return U;
    }

    /** @notice Copy of sendAsset with no calldata_ */
    function sendAsset(
        bytes32 channelId,
        bytes calldata toVault,
        bytes calldata toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser
    ) external override returns (uint256) {
        bytes memory calldata_ = new bytes(0);
        return
            sendAsset(
                channelId,
                toVault,
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
     * @param fromVault The source vault.
     * @param toAssetIndex Index of the asset to be purchased with Units.
     * @param toAccount The recipient.
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     * @param fromAmount Used to connect swaps cross-chain. The input amount on the sending chain.
     * @param fromAsset Used to connect swaps cross-chain. The input asset on the sending chain.
     * @param blockNumberMod Used to connect swaps cross-chain. The block number from the host side.
     */
    function receiveAsset(
        bytes32 channelId,
        bytes calldata fromVault,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod
    ) nonReentrant public override returns (uint256) {
        // Only allow connected vaults
        if (!_vaultConnection[channelId][fromVault]) revert VaultNotConnected(channelId, fromVault);
        // The chainInterface is the only valid caller of this function.
        require(msg.sender == _chainInterface);

        _updateWeights();

        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];

        // Check and update the security limit.
        _updateUnitCapacity(U);

        // Calculate the swap return value.
        // Fee is always taken on the sending token.
        uint256 purchasedTokens = calcReceiveAsset(toAsset, U);

        // Ensure the user is satisfied by the number of tokens.
        if (minOut > purchasedTokens) revert ReturnInsufficient(purchasedTokens, minOut);

        // Send the assets to the user.
        ERC20(toAsset).safeTransfer(toAccount, purchasedTokens);

        emit ReceiveAsset(
            channelId, 
            fromVault, 
            toAccount, 
            toAsset, 
            U, 
            purchasedTokens, 
            fromAmount,
            fromAsset,
            blockNumberMod
        );

        return purchasedTokens;
    }

    function receiveAsset(
        bytes32 channelId,
        bytes calldata fromVault,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod,
        address dataTarget,
        bytes calldata data
    ) external override returns (uint256) {
        uint256 purchasedTokens = receiveAsset(
            channelId,
            fromVault,
            toAssetIndex,
            toAccount,
            U,
            minOut,
            fromAmount,
            fromAsset,
            blockNumberMod
        );

        // Let users define custom logic which should be executed after the swap.
        // The logic is not contained within a try - except so if the logic reverts
        // the transaction will timeout and the user gets the input tokens on the sending chain.
        // If this is not desired, wrap further logic in a try - except at dataTarget.
        ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, data);
        // If dataTarget doesn't implement onCatalystCall BUT implements a fallback function, the call will still succeed.

        return purchasedTokens;
    }

    //--- Liquidity swapping ---//
    // Because of the way vault tokens work in a group of vaults, there
    // needs to be a way for users to easily get a distributed stake.
    // Liquidity swaps is a macro implemented on the smart contract level to:
    // 1. Withdraw tokens.
    // 2. Convert tokens to units & transfer to target vault.
    // 3. Convert units to an even mix of tokens.
    // 4. Deposit the even mix of tokens.
    // In 1 user invocation.

    /**
     * @notice Initiate a cross-chain liquidity swap by withdrawing tokens and converting them to units.
     * @dev While the description says tokens are withdrawn and then converted to units, vault tokens are converted
     * directly into units through the following equation:
     *      U = ln(PT/(PT-pt)) * \sum W_i
     * @param channelId The target chain identifier.
     * @param toVault The target vault on the target chain encoded in bytes32.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param vaultTokens The number of vault tokens to exchange
     * @param minOut An array of minout describing: [the minimum number of vault tokens, the minimum number of reference assets]
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address
     * @param calldata_ Data field if a call should be made on the target chain. 
     * Should be encoded abi.encode(<address>,<data>)
     * @return uint256 The number of units minted.
     */
    function sendLiquidity(
        bytes32 channelId,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint256 vaultTokens,
        uint256[2] calldata minOut,
        address fallbackUser,
        bytes memory calldata_
    ) nonReentrant public override returns (uint256) {
        // Only allow connected vaults
        if (!_vaultConnection[channelId][toVault]) revert VaultNotConnected(channelId, toVault);
        require(toVault.length == 65);  // dev: Vault addresses are uint8 + 64 bytes.
        require(toAccount.length == 65);  // dev: Account addresses are uint8 + 64 bytes.

        // Address(0) is not a valid fallback user. (As checking for escrow overlap
        // checks if the fallbackUser != address(0))
        require(fallbackUser != address(0));

        // Update weights
        _updateWeights();

        uint256 initialTotalSupply = totalSupply + _escrowedVaultTokens;
        // Since we have already cached totalSupply, we might as well burn the tokens
        // now. If the user doesn't have enough tokens, they save a bit of gas.
        _burn(msg.sender, vaultTokens);

        // Fetch wsum.
        uint256 wsum = _maxUnitCapacity / FixedPointMathLib.LN2;

        // Compute the unit value of the provided vaultTokens.
        // This step simplifies withdrawing and swapping into a single calculation.
        uint256 U = uint256(FixedPointMathLib.lnWad(  // uint256: ln computed of a value always > 1, hence always positive
            int256(FixedPointMathLib.divWadDown(initialTotalSupply, initialTotalSupply - vaultTokens))   // int256: if casting overflows, the result is a negative input. This reverts.
        )) * wsum;

        // Transfer the units to the target vaults.
        CatalystIBCInterface(_chainInterface).sendCrossChainLiquidity(
            channelId,
            toVault,
            toAccount,
            U,
            minOut,
            vaultTokens,
            calldata_
        );

        // Only need to hash info that is required by the escrow (+ some extra for randomisation)
        // No need to hash context (as token/liquidity escrow data is different), fromVault, toVault, targetAssetIndex, minOut, CallData
        bytes32 sendLiquidityHash = _computeSendLiquidityHash(
            toAccount,      // Ensures no collisions between different users
            U,              // Used to randomise the hash
            vaultTokens,     // Required! to validate release escrow data
            uint32(block.number) // May overflow, but this is desired (% 2**32)
        );

        // Escrow the vault tokens
        require(_escrowedVaultTokensFor[sendLiquidityHash] == address(0));
        _escrowedVaultTokensFor[sendLiquidityHash] = fallbackUser;
        _escrowedVaultTokens += vaultTokens;

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit at a low cost.

        emit SendLiquidity(
            channelId,
            toVault,
            toAccount,
            vaultTokens,
            minOut,
            U
        );

        return U;
    }

    /** @notice Copy of sendLiquidity with no calldata_ */
    function sendLiquidity(
        bytes32 channelId,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint256 vaultTokens,
        uint256[2] calldata minOut,
        address fallbackUser
    ) external override returns (uint256) {
        bytes memory calldata_ = new bytes(0);
        return sendLiquidity(
            channelId,
            toVault,
            toAccount,
            vaultTokens,
            minOut,
            fallbackUser,
            calldata_
        );
    }

    /**
     * @notice Completes a cross-chain liquidity swap by converting units to tokens and depositing.
     * @dev Called exclusively by the chainInterface.
     * While the description says units are converted to tokens and then deposited, units are converted
     * directly to vault tokens through the following equation:
     *      pt = PT · (1 - exp(-U/sum W_i))/exp(-U/sum W_i)
     * @param channelId The incoming connection identifier.
     * @param fromVault The source vault
     * @param toAccount The recipient of the vault tokens
     * @param U Number of units to convert into vault tokens.
     * @param minVaultTokens The minimum number of vault tokens to mint on target vault. Otherwise: Reject
     * @param minReferenceAsset The minimum number of reference asset the vaults tokens are worth. Otherwise: Reject
     * @param fromAmount Used to connect swaps cross-chain. The input amount on the sending chain.
     * @param blockNumberMod Used to connect swaps cross-chain. The block number from the host side.
     * @return uint256 Number of vault tokens minted to the recipient.
     */
    function receiveLiquidity(
        bytes32 channelId,
        bytes calldata fromVault,
        address toAccount,
        uint256 U,
        uint256 minVaultTokens,
        uint256 minReferenceAsset,
        uint256 fromAmount,
        uint32 blockNumberMod
    ) nonReentrant public override returns (uint256) {
        // The chainInterface is the only valid caller of this function.
        require(msg.sender == _chainInterface);
        // Only allow connected vaults
        if (!_vaultConnection[channelId][fromVault]) revert VaultNotConnected(channelId, fromVault);

        _updateWeights();

        // Check if the swap is according to the swap limits
        _updateUnitCapacity(U);

        // Fetch wsum.
        uint256 wsum = _maxUnitCapacity / FixedPointMathLib.LN2;

        // Use the arbitarty integral to compute mint %. It comes as WAD, multiply by totalSupply
        // and divide by WAD (mulWadDown) to get number of vault tokens.
        // On totalSupply. Do not add escrow amount, as higher amount results in a larger return.
        uint256 vaultTokens = FixedPointMathLib.mulWadDown(_calcPriceCurveLimitShare(U, wsum), totalSupply);

        // Check if more than the minimum output is returned.
        if (minVaultTokens > vaultTokens) revert ReturnInsufficient(vaultTokens, minVaultTokens);
        // Then check if the minimum number of reference assets is honoured.
        if (minReferenceAsset > 0) {
            // This is done by computing the reference balance through a locally observable method.
            // The balance0s are a point on the invariant. As such, another way of deriving balance0
            // is by finding a balance such that \prod balance0 = \prod balance**weight.
            // First, we need to find the localInvariant: 
            // \prod balance ** weight 
            // balance0 = (\prod_i balance_i ** weight_i)**(1/(\sum_j weights_j)).
            // \prod_l ((\prod_i balance_i ** weight_i)**(1/(\sum_j weights_j))**weight_l) = 
            // ((\prod_i balance_i ** weight_i)**(1/(\sum_j weights_j))**(\sum_l weight_l)) = 
            // (\prod_i balance_i ** weight_i)**((\sum_l weight_l))/(\sum_j weights_j)) =
            // (\prod_i balance_i ** weight_i)**1 = \prod_i balance_i ** weight_i
            // Thus balance0 is a point on the invariant.
            // It is computed as: balance0 = exp((\sum (ln(balance_i) * weight_i)/(\sum_j weights_j)).
            uint256 localInvariant= 0;
            // Computes \sum (ln(balance_i) * weight_i
            for (uint256 it; it < MAX_ASSETS;) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];
                uint256 balance = ERC20(token).balanceOf(address(this));
                localInvariant += uint256(FixedPointMathLib.lnWad( // uint256 casting: Since balance*FixedPointMathLib.WAD >= FixedPointMathLib.WAD, lnWad always returns more than 0.
                    int256(balance*FixedPointMathLib.WAD) // int256 casting: If it overflows and becomes negative, then the ln function fails.
                )) * weight; 

                unchecked {
                    it++;
                }
            }

            // Compute (\sum (ln(balance_i) * weight_i)/(\sum_j weights_j)
            unchecked {
                // wsum is not 0.
                localInvariant = localInvariant / wsum;
            }

            // Compute exp((\sum (ln(balance_i) * weight_i)/(\sum_j weights_j))
            uint256 referenceAmount = uint256(FixedPointMathLib.expWad( // uint256 casting: expWad cannot be negative.
                int256(localInvariant) // int256 casting: If this overflows, reference amount is 0 or almost 0. Thus it will never pass line 1076. Thus the casting is safe.
                // If we actually look at what the calculation here is: (prod balance**weight)**(1/sum weights), we observe that the result should be limited by
                // max (ERC20(i).balanceOf(address(this))). So it will never overflow.
            )) / FixedPointMathLib.WAD;

            // Find the fraction of the referenceAmount that the user owns.
            // Add escrow to ensure that even if all ongoing transaction revert, the user gets their expected amount.
            // Add vault tokens because they are going to be minted.
            referenceAmount = (referenceAmount * vaultTokens)/(totalSupply + _escrowedVaultTokens + vaultTokens);
            if (minReferenceAsset > referenceAmount) revert ReturnInsufficient(referenceAmount, minReferenceAsset);
        }

        // Mint vault tokens for the user.
        _mint(toAccount, vaultTokens);

        emit ReceiveLiquidity(channelId, fromVault, toAccount, U, vaultTokens, fromAmount, blockNumberMod);

        return vaultTokens;
    }

    
    function receiveLiquidity(
        bytes32 channelId,
        bytes calldata fromVault,
        address who,
        uint256 U,
        uint256 minVaultTokens,
        uint256 minReferenceAsset,
        uint256 fromAmount,
        uint32 blockNumberMod,
        address dataTarget,
        bytes calldata data
    ) external override returns (uint256) {
        uint256 purchasedVaultTokens = receiveLiquidity(
            channelId,
            fromVault,
            who,
            U,
            minVaultTokens,
            minReferenceAsset,
            fromAmount,
            blockNumberMod
        );

        // Let users define custom logic which should be executed after the swap.
        // The logic is not contained within a try - except so if the logic reverts
        // the transaction will timeout and the user gets the input tokens on the sending chain.
        // If this is not desired, wrap further logic in a try - except at dataTarget.
        ICatalystReceiver(dataTarget).onCatalystCall(purchasedVaultTokens, data);
        // If dataTarget doesn't implement onCatalystCall BUT implements a fallback function, the call will still succeed.

        return purchasedVaultTokens;
    }

    //-- Escrow Functions --//

    /** 
     * @notice Deletes and releases escrowed tokens to the vault and updates the security limit.
     * @dev Should never revert!  
     * The base implementation exists in CatalystVaultCommon. The function adds security limit
     * adjustment to the implementation to swap volume supported.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units purchased.
     * @param escrowAmount The number of tokens escrowed.
     * @param escrowToken The token escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function onSendAssetSuccess(
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) public override {
        // Execute common escrow logic.
        super.onSendAssetSuccess(toAccount, U, escrowAmount, escrowToken, blockNumberMod);

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the vault, we should expect that there is exactly
        // the inswapped amount of trust in the vault. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        unchecked {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UC = _usedUnitCapacity;
            // If UC < U and we do UC - U < 0 underflow => bad.
            if (UC > U) {
                _usedUnitCapacity = UC - U; // Does not underflow since _usedUnitCapacity > U.
            } else if (UC != 0) {
                // If UC == 0, then we shouldn't do anything. Skip that case.
                // when UC <= U => UC - U <= 0 => max(UC - U, 0) = 0
                _usedUnitCapacity = 0;
            }
        }
    }

    /** 
     * @notice Deletes and releases liquidity escrowed tokens to the vault and updates the security limit.
     * @dev Should never revert!
     * The base implementation exists in CatalystVaultCommon. The function adds security limit
     * adjustment to the implementation to swap volume supported.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The number of units acquired.
     * @param escrowAmount The number of vault tokens escrowed.
     * @param blockNumberMod The block number at which the swap transaction was commited (mod 32)
     */
    function onSendLiquiditySuccess(
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) public override {
        // Execute common escrow logic.
        super.onSendLiquiditySuccess(toAccount, U, escrowAmount, blockNumberMod);

        // Incoming swaps should be subtracted from the unit flow.
        // It is assumed if the router was fraudulent, that no-one would execute a trade.
        // As a result, if people swap into the vault, we should expect that there is exactly
        // the inswapped amount of trust in the vault. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        unchecked {
            // Calling timeout and then ack should not be possible. 
            // The initial lines deleting the escrow protects against this.
            uint256 UC = _usedUnitCapacity;
            // If UC < U and we do UC - U < 0 underflow => bad.
            if (UC > U) {
                _usedUnitCapacity = UC - U; // Does not underflow since _usedUnitCapacity > U.
            } else if (UC != 0) {
                // If UC == 0, then we shouldn't do anything. Skip that case.
                // when UC <= U => UC - U <= 0 => max(UC - U, 0) = 0
                _usedUnitCapacity = 0;
            }
        }
    }
}