//SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.19;

import { ERC20 } from 'solady/tokens/ERC20.sol';
import { SafeTransferLib } from 'solady/utils/SafeTransferLib.sol';
import { FixedPointMathLib } from "solady/utils/FixedPointMathLib.sol";

import { ICatalystChainInterface } from "./interfaces/ICatalystChainInterface.sol";
import { WADWAD } from "./utils/MathConstants.sol";
import { CatalystVaultCommon } from "./CatalystVaultCommon.sol";
import { IntegralsAmplified } from "./IntegralsAmplified.sol";
import { ICatalystReceiver} from "./interfaces/IOnCatalyst.sol";
import "./ICatalystV1Vault.sol";

/**
 * @title Catalyst: Fixed rate cross-chain swap vault.
 * @author Cata Labs Inc.
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
 * It is important that both setup(...) AND initializeSwapCurves(...) are called
 * in the same transaciton, as otherwise the vault is not fully finalised and
 * may be configured incorrectly by a third-party.
 *
 * If connected to a supported cross-chain interface, call
 * setConnection to connect the vault with vaults on other chains.
 *
 * Finally, call finishSetup to give up the creator's control over the vault. 
 * !If finishSetup is not called, the vault can be drained by the creator!
 */
contract CatalystVaultAmplified is CatalystVaultCommon, IntegralsAmplified {
    //--- Storage ---//
    
    /** 
     * @notice Adjustment used to remove a certain escrow amount from the balance * computation
     */
    mapping(address => uint256) public _underwriteEscrowMatchBalance0;

    //--- ERRORS ---//
    // Errors are defined in interfaces/ICatalystV1VaultErrors.sol

    //--- Config ---//

    /** @notice Minimum time parameter adjustments can be made over. */
    uint256 constant MIN_ADJUSTMENT_TIME = 7 days;

    /**
     * @dev  When the swap is a very small size of the vault, the
     * swaps returns slightly more. To counteract this, an additional 
     * fee slightly larger than the error is added. The below 
     * constants determines when this fee is added and the size.
     */
    uint256 constant SMALL_SWAP_RATIO = 1e12;
    uint256 constant SMALL_SWAP_RETURN = 95e16;

    // For other config options, see CatalystVaultCommon.sol

    //-- Variables --//
    int64 public _oneMinusAmp;
    int64 public _targetAmplification;
 
    /**
     * @dev To keep track of pool ownership, the vault needs to keep track of
     * the local unit balance. That is, do other vaults own or owe assets to this vault?
     */
    int256 public _unitTracker;

    constructor(address factory_, address mathlib_) payable CatalystVaultCommon(factory_, mathlib_) {}

    /**
     * @notice Configures an empty vault.
     * @dev The initial token amounts should have been sent to the vault before setup is called.
     * Since someone can call setup can claim the initial tokens, this needs to be
     * done atomically!
     *
     * If 0 of a token in assets is provided, the setup reverts.
     * @param assets A list of token addresses to be associated with the vault.
     * @param weights Weights brings the price into a true 1:1 swap. That is:
     * i_t \cdot W_i = j_t \cdot W_j \forall i, j when P_i(i_t) = P_j(j_t).
     * in other words, weights are used to compensate for the difference in decimals. (or non 1:1.)
     * @param amp Amplification factor. Should be < 10**18.
     * @param depositor The address to mint tokens to.
     */
    function initializeSwapCurves(
        address[] calldata assets,
        uint256[] calldata weights,
        uint64 amp,
        address depositor
    ) external override {
        // May only be invoked by the FACTORY. The factory only invokes this function for proxy contracts.
        require(msg.sender == FACTORY); // dev: swap curves may only be initialized once by the factory
        require(_tokenIndexing[0] == address(0)); // dev: swap curves may only be initialized once by the factory
        // Check that the amplification is correct.
        require(amp < FixedPointMathLib.WAD);  // dev: amplification not set correctly.
        // Note there is no need to check whether assets.length/weights.length are valid, as invalid arguments
        // will either cause the function to fail (e.g. if assets.length > MAX_ASSETS the assignment
        // to initialBalances[it] will fail) or will cause the vault to get initialized with an undesired state
        // (and the vault shouldn't be used by anyone until its configuration has been finalised). 
        // In any case, the factory does check for valid assets/weights arguments to prevent erroneous configurations.
        // Note Since assets.len != 0 is not checked, the initial depositor may invoke this function many times, resulting
        // in vault tokens being minted for the 'depositor' every time. This is not an issue, since 'INITIAL_MINT_AMOUNT' is
        // an arbitrary number; the value of the vault tokens is determined by the ratio of the vault asset balances and vault
        // tokens supply once setup has finalized. Furthermore, the vault should not be used until setup has finished and the
        // vault configuration has been verified.
        
        unchecked {
            // Amplification is stored as 1 - amp since most equations uses amp this way.
            _oneMinusAmp = int64(uint64(FixedPointMathLib.WAD - amp));
            _targetAmplification = int64(uint64(FixedPointMathLib.WAD - amp));
        }   

        // Compute the security limit.
        uint256[] memory initialBalances = new uint256[](MAX_ASSETS);
        uint256 maxUnitCapacity = 0;
        uint assetLength = assets.length;
        for (uint256 it; it < assetLength;) {

            address tokenAddress = assets[it];
            _tokenIndexing[it] = tokenAddress;

            uint256 weight = weights[it];
            require(weight != 0);  // dev: invalid 0-valued weight provided
            _weight[tokenAddress] = weight;

            // The contract expects the tokens to have been sent to it before setup is
            // called. Make sure the vault has more than 0 tokens.
            // Reverts if tokenAddress is address(0).
            // This contract uses safeTransferLib from Solady. When "safeTransfering", there is no
            // check for smart contract code. This could be an issue if non-tokens are allowed to enter
            // the pool, as then the pool could be expoited by later deploying an address to the addres.
            // The below check ensure that there is a token deployed to the contract.
            uint256 balanceOfSelf = ERC20(tokenAddress).balanceOf(address(this));
            require(balanceOfSelf != 0); // dev: 0 tokens provided in setup.
            initialBalances[it] = balanceOfSelf;

            maxUnitCapacity += weight * balanceOfSelf;

            unchecked {
                ++it;
            }
        }

        // The security limit is implemented as being 50% of the current balance. 
        // Since the security limit is evaluated after balance changes, the limit in
        // storage should be the current balance.
        _maxUnitCapacity = maxUnitCapacity;

        // Mint vault tokens to the vault creator.
        _mint(depositor, INITIAL_MINT_AMOUNT);

        emit VaultDeposit(depositor, INITIAL_MINT_AMOUNT, initialBalances);
    }

    /** 
     * @notice Returns the current cross-chain swap capacity. 
     * @dev Overwrites the common implementation because of the
     * differences as to how it is used. As a result, this always returns
     * half of the common implementation (or of _maxUnitCapacity)
     */
    function getUnitCapacity() public view override returns (uint256) {
        return super.getUnitCapacity() >> 1;  // Equal to divide by 2.
    }

    /**
     * @notice Re-computes the security limit incase funds have been sent to the vault.
     */
    function updateMaxUnitCapacity() external {
        uint256 maxUnitCapacity;
        for (uint256 it; it < MAX_ASSETS;) {
            address asset = _tokenIndexing[it];
            if (asset == address(0)) break;

            maxUnitCapacity += (ERC20(asset).balanceOf(address(this)) - _escrowedTokens[asset]) * _weight[asset];

            unchecked {
                ++it;
            }
        }
        _maxUnitCapacity = maxUnitCapacity;
    }

    //--- Swap integrals ---//

    // Inherited from the Integrals file.

    /**
     * @notice Computes the return of SendAsset excluding fees.
     * @dev Reverts if 'fromAsset' is not a token in the vault or if 
     * 'amount' and the vault asset balance are both 0.
     * Does not contain the swap fee.
     * @param fromAsset Address of the token to sell.
     * @param amount Amount of from token to sell.
     * @return uint256 Units.
     */
    function calcSendAsset(
        address fromAsset,
        uint256 amount
    ) public view override returns (uint256) {
        // A high => fewer units returned. Do not subtract the escrow amount
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 W = _weight[fromAsset];

        // If 'fromAsset' is not part of the vault (i.e. W is 0) or if 'amount' and 
        // the vault asset balance (i.e. 'A') are both 0 this will revert, since 0**p is 
        // implemented as exp(ln(0) * p) and ln(0) is undefined.
        uint256 U = _calcPriceCurveArea(amount, A, W, _oneMinusAmp);

        // If the swap is a very small portion of the vault add an additional fee. This covers mathematical errors.
        unchecked { //SMALL_SWAP_RATIO is not zero, and if U * SMALL_SWAP_RETURN overflows, less is returned to the user.
            // Also U * SMALL_SWAP_RETURN cannot overflow, since U depends heavily on amount/A. If this number is small (which it is in this case) then U is also "small".
            if (A/SMALL_SWAP_RATIO >= amount) return U * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;
        }
        
        return U;
    }

    /**
     * @notice Computes the output of ReceiveAsset excluding fees.
     * @dev Reverts if 'toAsset' is not a token in the vault.
     * Does not contain the swap fee.
     * @param toAsset Address of the token to buy.
     * @param U Number of units to convert.
     * @return uint256 Ppurchased tokens.
     */
    function calcReceiveAsset(
        address toAsset,
        uint256 U
    ) public view override returns (uint256) {
        // B low => fewer tokens returned. Subtract the escrow amount to decrease the balance.
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - _escrowedTokens[toAsset];
        uint256 W = _weight[toAsset];

        // If someone were to purchase a token that is not part of the vault on setup
        // they would just add value to the vault. We don't care about it.
        // However, it will revert since the solved integral contains U/W and when
        // W = 0 then U/W returns division by 0 error.
        return _calcPriceCurveLimit(U, B, W, _oneMinusAmp);
    }

    /**
     * @notice Computes the output of localSwap excluding fees.
     * @dev Implemented through _calcCombinedPriceCurves.
     * Reverts if either 'fromAsset' or 'toAsset' is not in the vault, or if the vault 'fromAsset'
     * balance and 'amount' are both 0.
     * Does not contain the swap fee.
     * @param fromAsset Address of the token to sell.
     * @param toAsset Address of the token to buy.
     * @param amount Amount of from token to sell for to token.
     * @return output Output denominated in toAsset.
     */
    function calcLocalSwap(
        address fromAsset,
        address toAsset,
        uint256 amount
    ) public view override returns (uint256 output) {
        uint256 A = ERC20(fromAsset).balanceOf(address(this));
        uint256 B = ERC20(toAsset).balanceOf(address(this)) - _escrowedTokens[toAsset];
        uint256 W_A = _weight[fromAsset];
        uint256 W_B = _weight[toAsset];
        int256 oneMinusAmp = _oneMinusAmp;

        output = _calcPriceCurveLimit(_calcPriceCurveArea(amount, A, W_A, oneMinusAmp), B, W_B, oneMinusAmp);

        // If the swap is a very small portion of the vault add an additional fee. This covers mathematical errors.
        unchecked { //SMALL_SWAP_RATIO is not zero, and if output * SMALL_SWAP_RETURN overflows, less is returned to the user
            if (A/SMALL_SWAP_RATIO >= amount) return output * SMALL_SWAP_RETURN / FixedPointMathLib.WAD;
        }
    }

    /**
     * @notice Deposits a user-configurable amount of tokens.
     * @dev The swap fee is imposed on deposits.
     * Requires approvals for all tokens within the vault.
     * It is advised that the deposit matches the vault's %token distribution.
     * Deposit is done by converting tokenAmounts into units and then using
     * the macro for units to vault tokens. (_calcPriceCurveLimitShare).
     * The elements of tokenAmounts correspond to _tokenIndexing[0...N].
     * @param tokenAmounts Array of the tokens amounts to be deposited.
     * @param minOut Minimum number of vault tokens to be minted.
     * @return vaultTokens Number of minted vault tokens.
     */
    function depositMixed(
        uint256[] memory tokenAmounts,
        uint256 minOut
    ) nonReentrant external override returns(uint256 vaultTokens) {
        // _updateAmplification();
        int256 oneMinusAmp = _oneMinusAmp;

        uint256 it_times_walpha_amped;

        // There is a Stack too deep issue in a later branch. To counteract this,
        // wab is stored short-lived. This requires letting U get negative.
        // As such, we define an additional variable called intU which is signed
        int256 intU;
        
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the vault should have If the price in the pool is 1:1.
        // walpha_0 is computed several times in this contract:
        // - DepositMixed
        // - WithdrawMixed
        // - WithdrawAll
        // - sendLiquidity
        // - receiveLiquidity
        // Since the implementation is very similar, it could be computed seperatly.
        // However, some of the implementations differ notably:
        // - DepositMixed: The for loop is reused for computing the value of incoming assets.
        // - WithdrawMixed: The for loop is used to cache tokenIndexed, effAssetBalances, and assetWeight.
        // - WithdrawAll: The for loop is used to cache tokenIndexed, effWeightAssetBalances.
        // - Both sendLiquidity and receiveLiquidity implements the reference computation: computeBalance0().
        // Before each implementation, there will be a short comment to describe how the implementation is different.
        {
            int256 weightedAssetBalanceSum = 0;
            uint256 assetDepositSum = 0;
            for (uint256 it; it < MAX_ASSETS;) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                uint256 weight = _weight[token];
                uint256 tokenAmount = tokenAmounts[it];
                {
                // Whenever balance0 is computed, the true balance should be used.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this));

                {
                    // wa^(1-k) is required twice. It is F(A) in the
                    // sendAsset equation and part of the wa_0^(1-k) calculation.
                    // If weightAssetBalance == 0, then this computation would fail. However since 0^(1-k) = 0, we can set it to 0.
                    int256 wab = 0;
                    if (weightAssetBalance != 0) {
                        // calculate balance 0 with the underwritten amount subtracted.
                        wab = FixedPointMathLib.powWad(
                            int256((weightAssetBalance - _underwriteEscrowMatchBalance0[token] * weight) * FixedPointMathLib.WAD),  // If casting overflows to a negative number, powWad fails
                            oneMinusAmp
                        );

                        // if wab == 0, there is no need to add it. So only add if != 0.
                        weightedAssetBalanceSum += wab;

                        wab = FixedPointMathLib.powWad(
                            int256(weightAssetBalance * FixedPointMathLib.WAD),  // If casting overflows to a negative number, powWad fails
                            oneMinusAmp
                        );
                    }
                    
                    // This line is the origin of the stack too deep issue.
                    // Moving intU += before this section would solve the issue but it is not possible since it would evaluate incorrectly.
                    // Save gas if the user provides no tokens, as the rest of the loop has no effect in that case
                    if (tokenAmount == 0) {
                        unchecked {
                            ++it;
                        }
                        continue;
                    }
                    
                    // int_A^{A+x} f(w) dw = F(A+x) - F(A).
                    unchecked {
                        // This is -F(A). Since we are subtracting first, U (i.e. intU) must be able to go negative.
                        // |intU| < weightedAssetBalanceSum since F(A+x) is added to intU in the lines after this.
                        intU -= wab;
                    }
                }
                
                // Add F(A+x).
                // This computation will not revert, since we know tokenAmount != 0.
                intU += FixedPointMathLib.powWad(
                    int256((weightAssetBalance + weight * tokenAmount) * FixedPointMathLib.WAD), // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );
                
                }
                assetDepositSum += tokenAmount * weight;
                
                SafeTransferLib.safeTransferFrom(
                    token,
                    msg.sender,
                    address(this),
                    tokenAmount
                );  // dev: Token withdrawal from user failed.

                unchecked {
                    ++it;
                }
            }
            // Increase the security limit by the amount deposited.
            _maxUnitCapacity += assetDepositSum;
            // Short term decrease the security limit by the amount deposited.
            // While one may assume _usedUnitCapacity < _maxUnitCapacity, this is not always the case. As such, this remains checked.
            _usedUnitCapacity += assetDepositSum;
            
            // Compute the reference liquidity.
            // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
            // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
            // _unitTracker < weightedAssetBalance0Sum
            unchecked {
                // weightedAssetBalanceSum - _unitTracker can overflow for negative _unitTracker.
                // The result will be correct once it is casted to uint256.
                it_times_walpha_amped = uint256(weightedAssetBalanceSum - _unitTracker); // By design, weightedAssetBalanceSum > _unitTracker
                // Notice that we are not dividing by it. That is because we would then later have to multiply by it.
            }
        }

        // Subtract fee from U (intU). This prevents people from using deposit and withdrawal as a method of swapping.
        // To reduce costs, the governance fee is not taken. As a result, swapping through deposit+withdrawal circumvents
        // the governance fee. No incentives align for traders to abuse this and is nativly disincentivised by the higher gas cost.
        // intU should not be negative. But in the case it is, the result is very bad. For safety, check it is above 0.
        require(intU >= 0); // dev: U needs to be positive, otherwise, the uint256 casting becomes too larger.
        unchecked {
            // U (intU) is generally small, so the below equation should not overflow.
            // If it does, it has to be (uint256(intU) * (FixedPointMathLib.WAD - _vaultFee)) that overflows.
            // In which case, something close to 0 will be returned. When divided by FixedPointMathLib.WAD
            // it will return 0. The casting to int256 is then 0.
            intU = int256(
                // intU shouldn't be negative but the above check ensures it is ALWAYS positive.
                (uint256(intU) * (FixedPointMathLib.WAD - _vaultFee))/FixedPointMathLib.WAD
            );
        }

        int256 oneMinusAmpInverse = WADWAD / oneMinusAmp;

        // On totalSupply(). Do not add escrow amount, as higher amount results in a larger return.
        vaultTokens = _calcPriceCurveLimitShare(uint256(intU), totalSupply(), it_times_walpha_amped, oneMinusAmpInverse); // uint256: intU is positive by design.

        // Check that the minimum output is honoured.
        if (minOut > vaultTokens) revert ReturnInsufficient(vaultTokens, minOut);

        // Mint the desired number of vault tokens to the user.
        _mint(msg.sender, vaultTokens);

        // Emit the deposit event
        emit VaultDeposit(msg.sender, vaultTokens, tokenAmounts);
    }

    /**
     * @notice Burns vault tokens and releases the symmetrical share of tokens to the burner.
     * This can impact the vault prices.
     * @dev This is the cheapest way to withdraw and only way to withdraw 100% of the liquidity.
     * @param vaultTokens Number of vault tokens to burn.
     * @param minOut Minimum token output. If less is returned, the transaction reverts.
     * @return amounts Array containing the amounts withdrawn.
     */
    function withdrawAll(
        uint256 vaultTokens,
        uint256[] memory minOut
    ) nonReentrant external override returns(uint256[] memory amounts) {
        // _updateAmplification();
        // Burn the desired number of vault tokens to the user.
        // If they don't have it, it saves gas.
        // * Remember to add vaultTokens when accessing totalSupply()
        _burn(msg.sender, vaultTokens);
        // (For everyone else, it is probably cheaper to burn last. However, burning here makes
        // the implementation more similar to the volatile one)

        int256 oneMinusAmp = _oneMinusAmp;

        // Cache weights and balances.
        address[MAX_ASSETS] memory tokenIndexed;
        uint256[MAX_ASSETS] memory effWeightAssetBalances;  // The 'effective' balances (compensated with the escrowed balances)

        uint256 walpha_0_ampped;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the vault should have If the price in the pool is 1:1.

        // This is a balance0 implementation. The for loop is used to cache tokenIndexed and effWeightAssetBalances.
        {
            int256 weightedAssetBalanceSum = 0;
            // The number of iterations, "it", is needed briefly outside the loop.
            uint256 it;
            for (it = 0; it < MAX_ASSETS;) {
                address token = _tokenIndexing[it];
                if (token == address(0)) break;
                tokenIndexed[it] = token;
                uint256 weight = _weight[token];

                // Whenever balance0 is computed, the true balance should be used.
                uint256 weightAssetBalance = weight * ERC20(token).balanceOf(address(this));

                // Since this is used for a withdrawal, the escrow amount needs to be subtracted to return less.
                effWeightAssetBalances[it] = weightAssetBalance - _escrowedTokens[token] * weight; // Store 

                // If weightAssetBalance == 0, then this computation would fail. However since 0^(1-k) = 0, we can set it to 0.
                if (weightAssetBalance != 0) {
                    int256 wab = FixedPointMathLib.powWad(
                        int256((weightAssetBalance - _underwriteEscrowMatchBalance0[token] * weight) * FixedPointMathLib.WAD),  // If casting overflows to a negative number, powWad fails
                        oneMinusAmp
                    );

                    // if wab == 0, there is no need to add it. So only add if != 0.
                    weightedAssetBalanceSum += wab;
                }

                unchecked {
                    ++it;
                }
            }

            // Compute the reference liquidity.
            // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
            // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
            // _unitTracker < weightedAssetBalance0Sum
            unchecked {
                // weightedAssetBalanceSum - _unitTracker can overflow for negative _unitTracker. The result will
                // be correct once it is casted to uint256.
                walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / it; // By design, weightedAssetBalanceSum > _unitTracker
            }
        }

        // For later event logging, the amounts transferred from the vault are stored.
        amounts = new uint256[](MAX_ASSETS);
        
        // The vault token to assets equation is:
        // wtk = wa ·(1 - ((wa^(1-k) - wa_0^(1-k) · (1 - (PT-pt)/PT)^(1-k))/wa^(1-k))^(1/(1-k))
        // The inner diff is wa_0^(1-k) · (1 - (PT-pt)/PT)^(1-k).
        // since it doesn't depend on the token, it should only be computed once.
        uint256 innerdiff;
        {
            // Remember to add the number of vault tokens burned to totalSupply()
            // _escrowedVaultTokens is added, since it makes pt_fraction smaller
            uint256 ts = (totalSupply() + _escrowedVaultTokens + vaultTokens);
            uint256 pt_fraction = ((ts - vaultTokens) * FixedPointMathLib.WAD) / ts;

            // If pt_fraction == 0 => 0^oneMinusAmp = powWad(0, oneMinusAmp) => exp(ln(0) * oneMinusAmp) which is undefined.
            // However, we know what 0^oneMinusAmp is: 0!. So we just set it to 0.
            innerdiff = pt_fraction == 0 ? walpha_0_ampped : FixedPointMathLib.mulWad(
                walpha_0_ampped, 
                    FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(  // Always casts a positive value
                    int256(pt_fraction),  // Casting always safe, as pt_fraction < 1
                    oneMinusAmp
                ))
            );
        }

        uint256 totalWithdrawn;
        int256 oneMinusAmpInverse = WADWAD / oneMinusAmp;
        for (uint256 it; it < MAX_ASSETS;) {
            address token = tokenIndexed[it];
            if (token == address(0)) break;

            // ampWeightAssetBalance cannot be cached because the balance0 computation does it without the escrow.
            // This computation needs to do it with the escrow.
            uint256 ampWeightAssetBalance = uint256(FixedPointMathLib.powWad(  // Powwad is always positive.
                int256(effWeightAssetBalances[it] * FixedPointMathLib.WAD),  // If casting overflows to a negative number, powWad fails
                oneMinusAmp
            ));
            //! If the vault doesn't have enough assets for a withdrawal, then
            //! withdraw all of the vaults assets. This should be protected against by setting minOut != 0.
            //! This happens because the vault expects assets to come back. (it is owed assets)
            //! We don't want to keep track of debt so we simply return less
            uint256 weightedTokenAmount = effWeightAssetBalances[it];
            //! The above happens if innerdiff >= ampWeightAssetBalance. So if that isn't
            //! the case, we should compute the true value.
            if (innerdiff < ampWeightAssetBalance) {
                // wtk = wa ·(1 - ((wa^(1-k) - wa_0^(1-k) · (1 - (PT-pt)/PT)^(1-k))/wa^(1-k))^(1/(1-k))
                // wtk = wa ·(1 - ((wa^(1-k) - innerdiff)/wa^(1-k))^(1/(1-k))
                // Since ampWeightAssetBalance ** (1/(1-amp)) == effWeightAssetBalances but the
                // mathematical lib returns ampWeightAssetBalance ** (1/(1-amp)) < effWeightAssetBalances.
                // the result is that if innerdiff isn't big enough to make up for the difference
                // the transaction reverts. If that is the case, use withdrawAll.
                // This quirk is "okay", since it means fewer tokens are always returned.

                // Since tokens are withdrawn, the change is negative. As such, multiply the equation by -1.
                weightedTokenAmount = FixedPointMathLib.mulWad(
                    weightedTokenAmount,
                    FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad(  // The inner is between 0 and 1. Power of < 1 is always between 0 and 1.
                        int256(FixedPointMathLib.divWadUp( // 0 < innerdiff < ampWeightAssetBalance => < 1 thus casting never overflows. 
                            ampWeightAssetBalance - innerdiff,
                            ampWeightAssetBalance
                        )),
                        oneMinusAmpInverse // 1/(1-amp)
                    ))
                );
            }

            // Store the amount withdrawn to subtract from the security limit later.
            totalWithdrawn += weightedTokenAmount;

            unchecked {
                // remove the weight from weightedTokenAmount.
                weightedTokenAmount /= _weight[token];
            }
            
            // Check if the user is satisfied with the output.
            uint256 tokenMinOut = minOut[it];   // GAS SAVING
            if (tokenMinOut > weightedTokenAmount) revert ReturnInsufficient(weightedTokenAmount, tokenMinOut);

            // Store the token amount.
            amounts[it] = weightedTokenAmount;

            // Transfer the released tokens to the user.
            SafeTransferLib.safeTransfer(token, msg.sender, weightedTokenAmount);

            unchecked {
                ++it;
            }
        }

        // Decrease the security limit by the amount withdrawn.
        _maxUnitCapacity -= totalWithdrawn;
        if (_usedUnitCapacity <= totalWithdrawn) {
            _usedUnitCapacity = 0;
        } else {
            unchecked {
                // We know: _usedUnitCapacity > totalWithdrawn.
                _usedUnitCapacity -= totalWithdrawn;
            }
        }

        // Emit the event
        emit VaultWithdraw(msg.sender, vaultTokens, amounts);
    }

    /**
     * @notice Burns vaultTokens and release a token distribution set by the user.
     * @dev It is advised that the withdrawal matches the vault's %token distribution.
     * Notice the special scheme for the ratios used. This is done to optimise gas since it doesn't require a sum or ratios.
     * Cannot be used to withdraw all liquidity. For that, withdrawAll should be used.
     * @param vaultTokens Number of vault tokens to withdraw.
     * @param withdrawRatio Percentage of units used to withdraw. In the following special scheme: U_0 = U · withdrawRatio[0], U_1 = (U - U_0) · withdrawRatio[1], U_2 = (U - U_0 - U_1) · withdrawRatio[2], .... Is WAD.
     * @param minOut Minimum number of tokens withdrawn.
     * @return amounts Array containing the amounts withdrawn.
     */
    function withdrawMixed(
        uint256 vaultTokens,
        uint256[] calldata withdrawRatio,
        uint256[] calldata minOut
    ) nonReentrant external override returns(uint256[] memory amounts) {
        // _updateAmplification();
        // Burn the desired number of vault tokens to the user. If they don't have it, it saves gas.
        // * Remember to add vaultTokens when accessing totalSupply()
        _burn(msg.sender, vaultTokens);
        // (For everyone else, it is probably cheaper to burn last. However, burning here makes
        // the implementation more similar to the volatile one).

        int256 oneMinusAmp = _oneMinusAmp;

        // Cache weights and balances.
        address[MAX_ASSETS] memory tokenIndexed;
        uint256[MAX_ASSETS] memory effAssetBalances; // The 'effective' balances (compensated with the escrowed balances)

        uint256 U = 0;
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the vault should have if the price in the pool is 1:1.
        // unlike in withdrawAll, this value is needed to compute U.
        {
            // As such, we don't need to remember the value beyond this section.
            uint256 walpha_0_ampped;

            // This is a balance0 implementation. The for loop is used to cache tokenIndexed, effAssetBalances and assetWeight.
            {
                int256 weightedAssetBalanceSum = 0;
                // A very careful stack optimisation is made here.
                // The number of iterations, "it", is needed briefly outside the loop.
                // To reduce the number of items in the stack, U = it.
                for (U = 0; U < MAX_ASSETS;) {
                    address token = _tokenIndexing[U];
                    if (token == address(0)) break;
                    tokenIndexed[U] = token;
                    uint256 weight = _weight[token];

                    // Whenever balance0 is computed, the true balance should be used.
                    uint256 ab = ERC20(token).balanceOf(address(this));

                    // Later we need to use the asset balances. Since it is for a withdrawal, we should subtract the escrowed tokens
                    // such that less is returned.
                    effAssetBalances[U] = ab - _escrowedTokens[token];

                    // subtract _underwriteEscrowMatchBalance0 since this is used for balance0.
                    uint256 weightAssetBalance = weight * (ab - _underwriteEscrowMatchBalance0[token]);

                    // If weightAssetBalance == 0, then this computation would fail. However since 0^(1-k) = 0, we can set it to 0.
                    int256 wab = 0;
                    if (weightAssetBalance != 0) {
                        wab = FixedPointMathLib.powWad(
                            int256(weightAssetBalance * FixedPointMathLib.WAD),  // If casting overflows to a negative number, powWad fails
                            oneMinusAmp
                        );

                        // if wab == 0, there is no need to add it. So only add if != 0.
                        weightedAssetBalanceSum += wab;
                    }

                    unchecked {
                        ++U;
                    }
                }

                // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
                // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
                // _unitTracker < weightedAssetBalance0Sum
                unchecked {
                    // weightedAssetBalanceSum - _unitTracker can overflow for negative _unitTracker. The result will
                    // be correct once it is casted to uint256.
                    walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / U; // By design, weightedAssetBalanceSum > _unitTracker
                }

                // set U = number of tokens in the vault. But that is exactly what it is.
            }
            // Remember to add the number of vault tokens burned to totalSupply()
            uint256 ts = totalSupply() + _escrowedVaultTokens + vaultTokens;
            // Since vault tokens are getting subtracted from the total supply, remember
            // to add a negative sign to vault tokens.
            uint256 pt_fraction = FixedPointMathLib.divWad(ts - vaultTokens, ts);

            // Compute the unit worth of the vault tokens.
            // Recall that U is equal to N already. So we only need to multiply by the right side.
            // Since pt_fraction < 1, the units are negative. This is expected for swap to tokens. As such
            // FixedPointMathLib.WAD is moved in front to make U positive.
            U *= FixedPointMathLib.mulWad(
                walpha_0_ampped, 
                FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad( // Always casts a positive value
                    int256(pt_fraction), // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                )) 
            );
        }

        // For later event logging, the amounts transferred to the vault are stored.
        amounts = new uint256[](MAX_ASSETS);
        uint256 totalWithdrawn;
        for (uint256 it; it < MAX_ASSETS;) {
            // Ideally we would collect the token into memory to save gas but there isn't space in the stack.
            if (tokenIndexed[it] == address(0)) break;

            // Units allocated for the specific token.
            uint256 U_i = FixedPointMathLib.mulWad(U, withdrawRatio[it]);
            if (U_i == 0) {
                // After a withdrawRatio of 1, all other withdrawRatios should be 0. Otherwise, there was an input error.
                if (withdrawRatio[it] != 0) revert WithdrawRatioNotZero();
                // Check the minimum output. This is important, since the normal check is skipped.
                if (minOut[it] != 0) revert ReturnInsufficient(0, minOut[it]);

                unchecked {
                    ++it;
                }
                continue;
            }
            U -= U_i;  // Subtract the number of units used. This will underflow for malicious withdrawRatios > 1.
            
            uint256 assetWeight = _weight[tokenIndexed[it]];
            // Units are shared between "liquidity units" and "token units". As such, we just need to convert the units to tokens.
            uint256 tokenAmount = _calcPriceCurveLimit(U_i, effAssetBalances[it], assetWeight, oneMinusAmp);

            // Ensure the output satisfies the user.
            if (minOut[it] > tokenAmount) revert ReturnInsufficient(tokenAmount, minOut[it]);

            // Store amount for withdraw event.
            amounts[it] = tokenAmount;

            // Transfer the released tokens to the user.
            SafeTransferLib.safeTransfer(tokenIndexed[it], msg.sender, tokenAmount);

            // Decrease the security limit by the amount withdrawn.
            totalWithdrawn += tokenAmount * assetWeight;

            unchecked {
                ++it;
            }
        }
        // Ensure all units are used. This should be done by setting at least one withdrawRatio to 1 (WAD).
        if (U != 0) revert UnusedUnitsAfterWithdrawal(U);
        
        // Decrease the security limit by the amount withdrawn.
        _maxUnitCapacity -= totalWithdrawn;
        if (_usedUnitCapacity <= totalWithdrawn) {
            _usedUnitCapacity = 0;
        } else {
            unchecked {
                // We know: _usedUnitCapacity > totalWithdrawn >= 0.
                _usedUnitCapacity -= totalWithdrawn;
            }
        }

        // Emit the event
        emit VaultWithdraw(msg.sender, vaultTokens, amounts);
    }

    /**
     * @notice A swap between 2 assets within the vault. Is atomic.
     * @param fromAsset Asset the user wants to sell.
     * @param toAsset Asset the user wants to buy.
     * @param amount Amount of fromAsset the user wants to sell.
     * @param minOut Minimum output the user wants. Otherwise, the transaction reverts.
     * @return out The number of tokens purchased.
     */
    function localSwap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) nonReentrant external override returns (uint256 out) {
        // _updateAmplification();
        uint256 fee = FixedPointMathLib.mulWad(amount, _vaultFee);

        // Calculate the return value.
        out = calcLocalSwap(fromAsset, toAsset, amount - fee);

        // Ensure the return value is more than the minimum output.
        if (minOut > out) revert ReturnInsufficient(out, minOut);

        // Transfer tokens to the user and collect tokens from the user.
        // The order doesn't matter, since the function is reentrant protected.
        // The transaction that is most likly to revert is first.
        SafeTransferLib.safeTransferFrom(fromAsset, msg.sender, address(this), amount);
        SafeTransferLib.safeTransfer(toAsset, msg.sender, out);

        // Collect potential governance fee
        _collectGovernanceFee(fromAsset, fee);

        // For amplified vaults, the security limit is based on the sum of the tokens in the vault.
        uint256 weightedAmount = amount * _weight[fromAsset];
        uint256 weightedOut = out * _weight[toAsset];
        // The if statement ensures the independent calculations never under or overflow.
        if (weightedOut > weightedAmount) {
            _maxUnitCapacity -= weightedOut - weightedAmount;
        } else {
            _maxUnitCapacity += weightedAmount - weightedOut;
        }

        emit LocalSwap(msg.sender, fromAsset, toAsset, amount, out);
    }

    /** @notice Common logic between sendAsset implementations */
    function _sendAsset(
        RouteDescription calldata routeDescription,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 U,
        uint256 amount,
        uint256 fee,
        uint256 minOut,
        address fallbackUser,
        uint16 underwriteIncentiveX16,
        bytes calldata calldata_
    ) internal {
        // Fallback user cannot be address(0) since this is used as a check for the existance of an escrow.
        // It would also be a silly fallback address.
        require(fallbackUser != address(0));
        // onSendAssetSuccess requires casting U to int256 to update the _unitTracker and must never revert. Check for overflow here.
        require(U < uint256(type(int256).max));  // int256 max fits in uint256
        _unitTracker += int256(U);

        // Send the purchased units to the target vault on the target chain.
        ICatalystChainInterface(_chainInterface).sendCrossChainAsset{value: msg.value}(
            routeDescription,
            toAssetIndex,
            U,
            minOut,
            amount - fee,
            fromAsset,
            underwriteIncentiveX16,
            calldata_
        );

        // Store the escrow information. For that, an index is required. Since we need this index twice, we store it.
        // Only information that is relevant for the escrow has to be hashed. (+ some extra for randomisation)
        // No need to hash context (as token/liquidity escrow data is different), fromVault, toVault, targetAssetIndex, minOut, CallData
        bytes32 sendAssetHash = _computeSendAssetHash(
            routeDescription.toAccount,              // Ensures no collisions between different users
            U,                      // Used to randomise the hash
            amount - fee,           // Required! to validate release escrow data
            fromAsset,              // Required! to validate release escrow data
            uint32(block.number)    // May overflow, but this is desired (% 2**32)
        );

        // Escrow the tokens used to purchase units. These will be sent back if transaction doesn't arrive / timeout.
        _setTokenEscrow(
            sendAssetHash,
            fallbackUser,
            fromAsset,
            amount - fee
        );
        // Notice that the fee is subtracted from the escrow. If this is not done, the escrow can be used as a cheap denial of service vector.
        // This is unfortunate.

        // Collect the tokens from the user.
        SafeTransferLib.safeTransferFrom(fromAsset, msg.sender, address(this), amount);

        // Governance Fee
        _collectGovernanceFee(fromAsset, fee);

        // Adjustment of the security limit is delayed until ack to avoid a router abusing timeout to circumvent the security limit.

        emit SendAsset(
            routeDescription.chainIdentifier,
            routeDescription.toVault,
            routeDescription.toAccount,
            fromAsset,
            toAssetIndex,
            amount,
            minOut,
            U,
            fee,
            underwriteIncentiveX16
        );
    }

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfering the units to the target vault.
     * @param routeDescription Cross-chain route description that contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param fromAsset Asset the user wants to sell.
     * @param toAssetIndex Index of the asset the user wants to buy in the target vault.
     * @param amount Number of fromAsset to sell to the vault.
     * @param minOut Minimum number output of tokens on the target chain.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @param underwriteIncentiveX16 Payment for underwriting the swap (out of type(uint16).max).
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: bytes.concat(bytes20(uint160(<address>)), <data>). At maximum 65535 bytes can be passed.
     * @return U Number of units bought.
     */
    function sendAsset(
        RouteDescription calldata routeDescription,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        uint16 underwriteIncentiveX16,
        bytes calldata calldata_
    ) nonReentrant onlyConnectedPool(routeDescription.chainIdentifier, routeDescription.toVault) external payable override returns (uint256 U) {
        // _updateAmplification();

        uint256 fee = FixedPointMathLib.mulWad(amount, _vaultFee);

        // Calculate the units bought.
        U = calcSendAsset(fromAsset, amount - fee);

        // Execute the common sendAsset logic.
        _sendAsset(
            routeDescription,
            fromAsset,
            toAssetIndex,
            U,
            amount,
            fee,
            minOut,
            fallbackUser,
            underwriteIncentiveX16,
            calldata_
        );
    }

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another vault using a fixed number of units.
     * @dev This function is intended to match an existing underwrite. Since normal sendAssets aren't "exact" in regards to U,
     * this functions makes it easier to hit a specific underwrite.
     * @param routeDescription Cross-chain route description that contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param fromAsset Asset the user wants to sell.
     * @param toAssetIndex Index of the asset the user wants to buy in the target vault.
     * @param amount Number of fromAsset to sell to the vault.
     * @param minOut Minimum number output of tokens on the target chain.
     * @param minU Minimum and exact number of units sent.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @param underwriteIncentiveX16 Payment for underwriting the swap (out of type(uint16).max).
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: bytes.concat(bytes20(uint160(<address>)), <data>). At maximum 65535 bytes can be passed..
     * @return U Always equal to minOut, as that is the number of units to be used on the destination chain.
     */
     function sendAssetFixedUnit(
        ICatalystV1Structs.RouteDescription calldata routeDescription,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        uint256 minU,
        address fallbackUser,
        uint16 underwriteIncentiveX16,
        bytes calldata calldata_
    ) nonReentrant onlyConnectedPool(routeDescription.chainIdentifier, routeDescription.toVault) external payable override returns (uint256 U) {
        // _updateAmplification();

        uint256 fee = FixedPointMathLib.mulWad(amount, _vaultFee);

        // Calculate the units bought.
        U = calcSendAsset(fromAsset, amount - fee);

        if (U < minU) revert ReturnInsufficient(U, minU);
        // The set number of units bought to minU.
        U = minU;

        // Execute the common sendAsset logic.
        _sendAsset(
            routeDescription,
            fromAsset,
            toAssetIndex,
            U,
            amount,
            fee,
            minOut,
            fallbackUser,
            underwriteIncentiveX16,
            calldata_
        );
    }

    /**
     * @notice Handles common logic associated with the completion of a cross-chain swap.
     * This function convert incoming Units (expected to be from an incoming cross-chain swap) into a specific token.
     * @dev This function is intended to finalise a receiveAsset call.
     * @param toAsset Asset to buy with the units.
     * @param U Incoming units to be turned into vaults tokens.
     * @param minOut Minimum number of tokens to purchase. Will revert if less.
     * @return purchasedTokens Number of toAsset bought.
     */
    function _receiveAsset(
        address toAsset,
        uint256 U,
        uint256 minOut
    ) internal override returns (uint256 purchasedTokens) {
        // _updateAmplification();

        // Calculate the swap return value. Fee is always taken on the sending token.
        purchasedTokens = calcReceiveAsset(toAsset, U);

        // Check if the swap is according to the swap limits.
        uint256 deltaSecurityLimit = purchasedTokens * _weight[toAsset];
        if (_maxUnitCapacity <= deltaSecurityLimit) revert ExceedsSecurityLimit();
        unchecked {
            // We know that _maxUnitCapacity > deltaSecurityLimit so it cannot underflow.
            _maxUnitCapacity -= deltaSecurityLimit;
        }
        _updateUnitCapacity(deltaSecurityLimit);

        // Ensure the user is satisfied with the number of tokens.
        if (minOut > purchasedTokens) revert ReturnInsufficient(purchasedTokens, minOut);

        // Track units for balance0 computation.
        _unitTracker -= int256(U);
    }

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token.
     * @dev Security checks are performed by _receiveAsset.
     * @param channelId Source chain identifier.
     * @param fromVault Source vault.
     * @param toAssetIndex Index of the asset to be purchased.
     * @param toAccount Recipient of assets on destination chain.
     * @param U Incoming units.
     * @param minOut Minimum number of token to buy. Reverts back to the sending side.
     * @param fromAmount Used to match cross-chain swap events. The input amount minus fees on the sending chain.
     * @param fromAsset Used to match cross-chain swap events. The input asset on the source chain.
     * @param blockNumberMod Used to match cross-chain swap events. The block number from the source chain.
     * @param purchasedTokens Number of toAsset bought.
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
    ) nonReentrant onlyChainInterface onlyConnectedPool(channelId, fromVault) external override returns(uint256 purchasedTokens) {
        // Convert the asset index (toAsset) into the asset to be purchased.
        address toAsset = _tokenIndexing[toAssetIndex];
        purchasedTokens = _receiveAsset(
            toAsset,
            U,
            minOut
        );

        // Send the assets to the user.
        SafeTransferLib.safeTransfer(toAsset, toAccount, purchasedTokens);

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
    }

    //--- Liquidity swapping ---//
    // Because of the way vault tokens work in a pool, there needs to be a way for users to easily get
    // a distributed stake. Liquidity swaps is a macro implemented at the smart contract level equivalent to:
    // 1. Withdraw tokens.
    // 2. Convert tokens to units & transfer to target vault.
    // 3. Convert units to an even mix of tokens.
    // 4. Deposit the even mix of tokens.
    // In 1 user invocation.

    /** 
     * @notice Computes balance0**(1-amp) without any special caching.
     * @dev Whenever balance0 is computed, the true balance should be used instead of the one
     * modifed by the escrow. This is because balance0 is constant during swaps. Thus, if the
     * balance was modified, it would not be constant during swaps.
     * The function also returns the vault asset count as it is always used in conjunction with walpha_0_ampped.
     * The external function does not.
     * @return walpha_0_ampped Balance0**(1-amp)
     * @return it the vault asset count
     */
    function _computeBalance0(int256 oneMinusAmp) internal view returns(uint256 walpha_0_ampped, uint256 it) {
        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the vault should have IF the price in the pool is 1:1.

        // This is a balance0 implementation. The balance 0 implementation here is reference.
        int256 weightedAssetBalanceSum = 0;
        for (it; it < MAX_ASSETS;) {
            address token = _tokenIndexing[it];
            if (token == address(0)) break;
            uint256 weight = _weight[token];

            uint256 weightAssetBalance = weight * (ERC20(token).balanceOf(address(this)) - _underwriteEscrowMatchBalance0[token]);

            // If weightAssetBalance == 0, then this computation would fail. However since 0^(1-k) = 0, we can set it to 0.
            int256 wab = 0;
            if (weightAssetBalance != 0){
                wab = FixedPointMathLib.powWad(
                    int256(weightAssetBalance * FixedPointMathLib.WAD),     // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                );

                // if wab == 0, there is no need to add it. So only add if != 0.
                weightedAssetBalanceSum += wab;
            }

            unchecked {
                ++it;
            }
        }

        // weightedAssetBalanceSum > _unitTracker always, since _unitTracker correlates to exactly
        // the difference between weightedAssetBalanceSum and weightedAssetBalance0Sum and thus
        // _unitTracker < weightedAssetBalance0Sum
        unchecked {
            // weightedAssetBalanceSum - _unitTracker can overflow for negative _unitTracker. The result will
            // be correct once it is casted to uint256.
            walpha_0_ampped = uint256(weightedAssetBalanceSum - _unitTracker) / it;   // By design, weightedAssetBalanceSum > _unitTracker
        }   
    }

    /** 
     * @notice Computes balance0 for the pool.
     * @dev This can be used as a local invariant. Is constant (or slowly increasing) for swaps.
     * Deposits and withdrawals change balance0 and as such, it cannot be used to examine if a vault is secure
     * by it self.
     * This function does not return balance0 as it, is returns a weighted amplified form.
     * For pretty much any real world usage of the function, this is the relevant form.
     * @return walpha_0 Balance0**(1-amp)
     */
    function computeBalance0() external view returns(uint256 walpha_0) {
        int256 oneMinusAmp = _oneMinusAmp;
       (uint256 walpha_0_ampped, ) = _computeBalance0(oneMinusAmp);

        walpha_0 = uint256( // casting: powWad is not negative.
            FixedPointMathLib.powWad(
                int256(walpha_0_ampped),  // Casting: If overflow, then powWad fails as the overflow is into negative.
                WADWAD / oneMinusAmp
            )
        );
    }

    /**
     * @notice Initiate a cross-chain liquidity swap by withdrawing tokens and converting them to units.
     * @dev While the description says tokens are withdrawn and then converted to units, vault tokens are converted
     * directly into units through the following equation: U = N · wa^(1-k) · (((PT + pt)/PT)^(1-k) - 1)
     * @param routeDescription Cross-chain route description that contains the chainIdentifier, toAccount, toVault, and relaying incentive.
     * @param vaultTokens Number of vault tokens to exchange.
     * @param minOut Array of minout describing: [the minimum number of vault tokens, the minimum number of reference assets].
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: abi.encodePacket(bytes20(<address>), <data>). At maximum 65535 bytes can be passed.
     * @return U Number of units bought.
     */
    function sendLiquidity(
        RouteDescription calldata routeDescription,
        uint256 vaultTokens,
        uint256[2] calldata minOut,
        address fallbackUser,
        bytes calldata calldata_
    ) nonReentrant onlyConnectedPool(routeDescription.chainIdentifier, routeDescription.toVault) external payable override returns (uint256 U) {
        // Fallback user cannot be address(0) since this is used as a check for the existance of an escrow.
        // It would also be a silly fallback address.
        require(fallbackUser != address(0));
        // Correct address format is checked on the cross-chain interface.

        // _updateAmplification();

        // When accesssing totalSupply, remember that we already burnt the incoming vaultTokens.
        _burn(msg.sender, vaultTokens);

        int256 oneMinusAmp = _oneMinusAmp;

        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the vault should have If the price in the pool is 1:1.
        (uint256 walpha_0_ampped, uint256 it) = _computeBalance0(oneMinusAmp);

        {
            // Plus _escrowedVaultTokens since we want the withdrawal to return less. Adding vaultTokens as these have already been burnt.
            uint256 ts = totalSupply() + _escrowedVaultTokens + vaultTokens;
            uint256 pt_fraction = FixedPointMathLib.divWad(ts + vaultTokens, ts);

            U = it * FixedPointMathLib.mulWad(
                walpha_0_ampped, 
                uint256(FixedPointMathLib.powWad( // Always casts a positive value
                    int256(pt_fraction), // If casting overflows to a negative number, powWad fails
                    oneMinusAmp
                )) - FixedPointMathLib.WAD
            );
            // onSendLiquiditySuccess requires casting U to int256 to update the _unitTracker and must never revert. Check for overflow here.
            require(U < uint256(type(int256).max)); // int256 max fits in uint256
            _unitTracker += int256(U);
        }

        // Transfer the units to the target vault.
        ICatalystChainInterface(_chainInterface).sendCrossChainLiquidity{value: msg.value}(
            routeDescription,
            U,
            minOut,
            vaultTokens,
            calldata_
        );

        // Store the escrow information. For that, an index is required. Since we need this index twice, we store it.
        // Only information that is relevant for the escrow has to be hashed. (+ some extra for randomisation)
        // No need to hash context (as token/liquidity escrow data is different), fromVault, toVault, targetAssetIndex, minOut, CallData
        bytes32 sendLiquidityHash = _computeSendLiquidityHash(
            routeDescription.toAccount,              // Ensures no collisions between different users
            U,                      // Used to randomise the hash
            vaultTokens,            // Required! to validate release escrow data
            uint32(block.number)    // May overflow, but this is desired (% 2**32)
        );

        // Emit event before setting escrow to clear up variables from stack.
        emit SendLiquidity(
            routeDescription.chainIdentifier,
            routeDescription.toVault,
            routeDescription.toAccount,
            vaultTokens,
            minOut,
            U
        );

        // Escrow the vault token used to purchase units. These will be sent back if transaction doesn't arrive / timeout.
        _setLiquidityEscrow(
            sendLiquidityHash,
            fallbackUser,
            vaultTokens
        );

        // Adjustment of the security limit is delayed until ack to avoid
        // a router abusing timeout to circumvent the security limit at a low cost.
    }

    /**
     * @notice Handles common logic assocaited with the completion of a cross-chain liquidity swap.
     * This function convert incoming units directly to vault tokens.
     * @dev This function is meant to finalise a receiveLiquidity call.
     * @param U Incoming units to be turned into vault tokens.
     * @param minVaultTokens Minimum number of vault tokens to mint (revert if less).
     * @param minReferenceAsset Minimum number of reference tokens the vaults tokens are worth (revert if less).
     * @return vaultTokens Minted vault tokens.
     */
    function _receiveLiquidity(
        uint256 U,
        uint256 minVaultTokens,
        uint256 minReferenceAsset
    ) internal returns (uint256 vaultTokens) {
        // _updateAmplification();

        int256 oneMinusAmp = _oneMinusAmp;

        // Compute walpha_0 to find the reference balances. This lets us evaluate the
        // number of tokens the vault should have If the price in the pool is 1:1.
        (uint256 walpha_0_ampped, uint256 it) = _computeBalance0(oneMinusAmp);

        int256 oneMinusAmpInverse = WADWAD / oneMinusAmp;

        uint256 it_times_walpha_amped = it * walpha_0_ampped;

        // On totalSupply(). Do not add escrow amount, as higher amount results in a larger return.
        vaultTokens = _calcPriceCurveLimitShare(U, totalSupply(), it_times_walpha_amped, oneMinusAmpInverse);

        // Check if more vault tokens than the minimum can be minted.
        if (minVaultTokens > vaultTokens) revert ReturnInsufficient(vaultTokens, minVaultTokens);
        // Then check if the minimum number of reference assets is honoured.
        if (minReferenceAsset != 0) {
            uint256 walpha_0 = uint256(FixedPointMathLib.powWad(  // uint256 casting: Is always positive.
                int256(walpha_0_ampped), // int256 casting: If casts to a negative number, powWad fails because it uses ln which can't take negative numbers.
                oneMinusAmpInverse
            )); 
            // Add escrow to ensure that even if all ongoing transaction revert, the user gets their expected amount.
            // Add vault tokens because they are going to be minted (We want the reference value after mint).
            uint256 walpha_0_owned = ((walpha_0 * vaultTokens) / (totalSupply() + _escrowedVaultTokens + vaultTokens)) / FixedPointMathLib.WAD;
            if (minReferenceAsset > walpha_0_owned) revert ReturnInsufficient(walpha_0_owned, minReferenceAsset);
        }

        // Update the unit tracker:
        _unitTracker -= int256(U);

        // Security limit
        {
            // To calculate the vaultTokenEquiv, we set \alpha_t = \alpha_0.
            // This should be a close enough approximation.
            // If U > it_times_walpha_amped, then U can purchase more than 50% of the vault.
            // And the below calculation doesn't work.
            if (it_times_walpha_amped <= U) revert ExceedsSecurityLimit();
            uint256 vaultTokenEquiv = FixedPointMathLib.mulWadUp(
                uint256(FixedPointMathLib.powWad( // Always casts a positive value
                    int256(it_times_walpha_amped), // If casting overflows to a negative number, powWad fails
                    oneMinusAmpInverse
                )),
                FixedPointMathLib.WAD - uint256(FixedPointMathLib.powWad( // powWad is always <= 1, as 'base' is always <= 1
                    int256(FixedPointMathLib.divWad( // Casting never overflows, as division result is always <= 1
                        it_times_walpha_amped - U,
                        it_times_walpha_amped
                    )),
                    oneMinusAmpInverse
                ))
            );
            // Check if the swap is according to the swap limits
            _updateUnitCapacity(FixedPointMathLib.mulWad(2, vaultTokenEquiv));
        }
    }

    /**
     * @notice Completes a cross-chain liquidity swap by converting units to tokens and depositing.
     * @dev Security checks are performed by _receiveLiquidity.
     * While the description says units are converted to tokens and then deposited, units are converted
     * directly to vault tokens through the following equation: pt = PT · (((N · wa_0^(1-k) + U)/(N · wa_0^(1-k))^(1/(1-k)) - 1)
     * @param channelId Source chain identifier.
     * @param fromVault Source vault.
     * @param toAccount Recipient of vault tokens.
     * @param U Incoming units to be turned into vault tokens.
     * @param minVaultTokens Minimum number of vault tokens to mint (revert if less).
     * @param minReferenceAsset Minimum number of reference tokens the vaults tokens are worth (revert if less).
     * @param fromAmount Used to match cross-chain swap events. The input amount on the source chain.
     * @param blockNumberMod Used to match cross-chain swap events. The block number from the source chain.
     * @return purchasedVaultTokens Minted vault tokens.
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
    ) nonReentrant onlyChainInterface onlyConnectedPool(channelId, fromVault) external override returns(uint256 purchasedVaultTokens) {
        purchasedVaultTokens = _receiveLiquidity(
            U,
            minVaultTokens,
            minReferenceAsset
        );

        emit ReceiveLiquidity(channelId, fromVault, toAccount, U, purchasedVaultTokens, fromAmount, blockNumberMod);

        // Mint vault tokens for the user.
        _mint(toAccount, purchasedVaultTokens);
    }

    //-- Escrow Functions --//

    /** 
     * @notice Deletes and releases escrowed tokens to the vault and updates the security limit.
     * @dev Should never revert!
     * The base implementation exists in CatalystVaultCommon. The function adds security limit
     * adjustment to the implementation to swap volume supported.
     * @param toAccount Recipient of the transaction on the target chain.
     * @param U Number of units purchased.
     * @param escrowAmount Number of tokens escrowed.
     * @param escrowToken Token escrowed.
     * @param blockNumberMod Block number at which the swap transaction was commited (mod 32)
     */
    function onSendAssetSuccess(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) public override {
        // Execute common escrow logic.
        super.onSendAssetSuccess(channelId, toAccount, U, escrowAmount, escrowToken, blockNumberMod);

        // Received assets should be subtracted from the used unit capacity.
        // It is assumed if the router was fraudulent no-one would execute a trade.
        // As a result, if people swap into the vault, we should expect that there is exactly
        // the inswapped amount of trust in the vault. If this wasn't implemented, there would be
        // a maximum daily cross chain volume, which is bad for liquidity providers.
        unchecked {
            // escrowAmount * _weight[escrowToken] has been calculated before.
            uint256 escrowAmountTimesWeight = escrowAmount * _weight[escrowToken];

            uint256 UC = _usedUnitCapacity;
            // If UC < escrowAmount and we do UC - escrowAmount < 0 underflow => bad.
            if (UC > escrowAmountTimesWeight) {
                _usedUnitCapacity = UC - escrowAmountTimesWeight; // Does not underflow since _usedUnitCapacity > escrowAmount.
            } else if (UC != 0) {
                // If UC == 0, then we shouldn't do anything. Skip that case.
                // when UC <= escrowAmount => UC - escrowAmount <= 0 => max(UC - escrowAmount, 0) = 0
                _usedUnitCapacity = 0;
            }

            // There is a chance that _maxUnitCapacity + escrowAmount * _weight[escrowToken] will overflow.
            // since the number has never been calculated before. This function should never revert so the computation
            // has to be done unchecked.
            uint256 muc = _maxUnitCapacity;
            uint256 new_muc = muc + escrowAmountTimesWeight; // Might overflow. Can be checked by comparing it against MUC.

            // If new_muc < muc, then new_muc has overflown. As a result, we should set muc = uint256::MAX
            if (new_muc < muc) {
                _maxUnitCapacity = type(uint256).max;
            } else {
                _maxUnitCapacity = new_muc;
            }
        }
    }

    /** 
     * @notice Deletes and releases escrowed tokens to the vault and updates the security limit.
     * @dev Should never revert!
     * The base implementation exists in CatalystVaultCommon. The function adds security limit
     * adjustment to the implementation to swap volume supported.
     * @param toAccount Recipient of the transaction on the target chain.
     * @param U Number of units acquired.
     * @param escrowAmount Number of tokens escrowed.
     * @param escrowToken Token escrowed.
     * @param blockNumberMod Block number at which the swap transaction was commited (mod 32)
     */
    function onSendAssetFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    ) public override {
        // Execute common escrow logic.
        super.onSendAssetFailure(channelId, toAccount, U, escrowAmount, escrowToken, blockNumberMod);

        // Removed timed-out units from the unit tracker. This will keep the
        // balance0 in balance, since tokens also leave the vault
        _unitTracker -= int256(U);  // It has already been checked on sendAsset that casting to int256 will not overflow.
                                    // Cannot be manipulated by the router as, otherwise, the swapHash check will fail
    }

    // onSendLiquiditySuccess is not overwritten since we are unable to increase
    // the security limit. This is because it is very expensive to compute the update
    // to the security limit. If someone liquidity swapped a significant amount of assets
    // it is assumed the vault has low liquidity. In these cases, liquidity swaps shouldn't be used.

    /** 
     * @notice Deletes and releases liquidity escrowed tokens to the vault and updates the security limit.
     * @dev Should never revert!  
     * The base implementation exists in CatalystVaultCommon.
     * @param toAccount Recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U Number of units initially acquired.
     * @param escrowAmount Number of vault tokens escrowed.
     * @param blockNumberMod Block number at which the swap transaction was commited (mod 32)
     */
    function onSendLiquidityFailure(
        bytes32 channelId,
        bytes calldata toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    ) public override {
        super.onSendLiquidityFailure(channelId, toAccount, U, escrowAmount, blockNumberMod);

        // Removed timed-out units from the unit tracker. This will keep the
        // balance0 in balance, since tokens also leave the vault
        _unitTracker -= int256(U);  // It has already been checked on sendAsset that casting to int256 will not overflow.
                                    // Cannot be manipulated by the router as, otherwise, the swapHash check will fail
    }

    function underwriteAsset(
        bytes32 identifier,
        address toAsset,
        uint256 U,
        uint256 minOut
    ) override public returns (uint256 purchasedTokens) {
        // We need to ensure that the conversion in deleteUnderwrite (and receiveAsset) doesn't overflow.
        require(U < uint256(type(int256).max));  // int256 max fits in uint256
        
        purchasedTokens = super.underwriteAsset(identifier, toAsset, U, minOut);

        // unit tracking is handled by _receiveAsset.

        // since _unitTrakcer -= int256(U) has been set but no tokens have
        // let the pool, we need to remove the corresponding amount from
        // the balance 0 computation to ensure it is still correct.
        unchecked {
            // Must be less than the vault balance.
            _underwriteEscrowMatchBalance0[toAsset] += purchasedTokens;
        }
    }

    function releaseUnderwriteAsset(
        address refundTo,
        bytes32 identifier,
        uint256 escrowAmount,
        address escrowToken,
        bytes32 sourceIdentifier,
        bytes calldata fromVault
    )  override public {
         super.releaseUnderwriteAsset(refundTo, identifier, escrowAmount, escrowToken, sourceIdentifier, fromVault);

        unchecked {
            _underwriteEscrowMatchBalance0[escrowToken] -= escrowAmount;
        }
    }

    function deleteUnderwriteAsset(
        bytes32 identifier,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken
    ) override public {
        super.deleteUnderwriteAsset(identifier, U, escrowAmount, escrowToken);
        
        // update the unit tracker. When underwriteAsset was called, the _unitTracker was updated with
        // _unitTrakcer -= int256(U) so we need to cancel that.
        _unitTracker += int256(U);  // It has already been checked on sendAsset that casting to int256 will not overflow.
                                    // Cannot be manipulated by the router as, otherwise, the swapHash check will fail

        unchecked {
            _underwriteEscrowMatchBalance0[escrowToken] -= escrowAmount;
        }
    }
}
