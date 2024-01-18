//SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

/// @title Events emitted by Catalyst v1 Vaults
/// @notice Contains all events emitted by the vault
/// @dev When using events to match transations, the combination of: channelId, fromVault, toAccount, toAsset, units, and block number is semi-guranteed to be unique.
///     If more than 2**32 blocks exist, then all instances are guaranteed to be non-overlapping
interface ICatalystV1VaultEvents {
    /**
     * @notice  Describes an atomic swap between the 2 tokens: _fromAsset and _toAsset.
     * @param account The user / exchange who facilitated the trade (msg.sender)
     * @param fromAsset The asset which was sold in exchange for _toAsset
     * @param toAsset The asset which was purchased with _fromAsset
     * @param fromAmount The number of _fromAsset sold
     * @param toAmount The number of tokens provided to toAccount
     */
    event LocalSwap(
        address indexed account,
        address fromAsset,
        address toAsset,
        uint256 fromAmount,
        uint256 toAmount
    );

    /**
     * @notice Describes the creation of an external swap: Cross-chain swap.
     * @param channelId The target chain identifier
     * @param toVault The target vault.
     * @param toAccount The recipient of the trade. The person who bought the trade is not present.
     * @param fromAsset The asset which was sold in exchange for _toAsset.
     * @param toAssetIndex The token index of the asset to purchase on _toChain.
     * @param fromAmount The number of _fromAsset sold.
     * @param minOut The minimum output to be accepted of fromAsset.
     * @param units The calculated number of units bought. Will be sold to buy _toAsset
     * @param underwriteIncentiveX16 The incentive out of 2**16 - 1 provided to the underwriter.
     * @param fee The number of tokens paid to the vault in fees.
     */
    event SendAsset(
        bytes32 channelId,
        bytes toVault,
        bytes toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 fromAmount,
        uint256 minOut,
        uint256 units,
        uint256 fee,
        uint16 underwriteIncentiveX16
    );

    /**
     * @notice Describes the arrival of an external swap: Cross-chain swap.
     * If toAccount is used to match trades, remember to convert it into 64 + 1 bytes.
     * @param channelId The target chain identifier
     * @param fromVault The source vault.
     * @param toAccount The recipient of the trade.
     * @param toAsset The asset which was purchased with _fromAsset
     * @param units The number of units sent from the other chain.
     * @param toAmount The number of tokens provided to toAccount
     * @param fromAmount The amount spent to get units on the source side.
     * @param fromAsset The provided asset on the source side.
     * @param sourceBlockNumberMod The block number of the sending transaction mod 2**32 - 1
     */
    event ReceiveAsset(
        bytes32 channelId,
        bytes fromVault,
        address toAccount,
        address toAsset,
        uint256 units,
        uint256 toAmount,
        uint256 fromAmount,
        bytes fromAsset,
        uint32 sourceBlockNumberMod

    );

    /**
     * @notice Describes the creation of a liquidity swap
     * @param channelId The target chain identifier
     * @param toVault The target vault.
     * @param toAccount The recipient of the liquidity. The person who bought the trade is not present.
     * @param fromAmount The number of _fromAsset sold
     * @param minOut An array containing a list of minimum outputs [minVaultTokens, minReferenceAssets]
     * @param units The calculated number of liquidity units bought.
     */
    event SendLiquidity(
        bytes32 channelId,
        bytes toVault,
        bytes toAccount,
        uint256 fromAmount,
        uint256[2] minOut,
        uint256 units
    );

    /**
     * @notice Describes the arrival of a liquidity swap
     * @param channelId The target chain identifier
     * @param fromVault The source vault.
     * @param toAccount The recipient of the liquidity.
     * @param units The number of liquidity units sent from the other chain.
     * @param toAmount The number of vault tokens provided to toAccount
     * @param fromAmount The amount spent to get units on the source side.
     * @param sourceBlockNumberMod The block number of the sending transaction mod 2**32 - 1
     */
    event ReceiveLiquidity(
        bytes32 channelId,
        bytes fromVault,
        address toAccount,
        uint256 units,
        uint256 toAmount,
        uint256 fromAmount,
        uint256 sourceBlockNumberMod
    );

    /**
     * @notice Emitted on liquidity deposits.
     * @param toAccount The depositor. Is credited with _mints vault tokens.
     * @param mint The number of minted vault tokens credited to toAccount
     * @param assets An array of the number of deposited assets.
     */
    event VaultDeposit(address indexed toAccount, uint256 mint, uint256[] assets);

    /**
     * @notice Emitted on liquidity withdrawal.
     * @param toAccount The withdrawer. Is debited _burns vault tokens.
     * @param burn The number of burned vault tokens.
     * @param assets An array of the token amounts returned
     */
    event VaultWithdraw(address indexed toAccount, uint256 burn, uint256[] assets);

    /** @notice Called upon successful asset swap. */
    event SendAssetSuccess(
        bytes32 channelId,
        bytes toAccount,
        uint256 units,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    );

    /** @notice Called upon failed asset swap. */
    event SendAssetFailure(
        bytes32 channelId,
        bytes toAccount,
        uint256 units,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    );

    /** @notice Called upon successful liquidity swap. */
    event SendLiquiditySuccess(
        bytes32 channelId,
        bytes toAccount,
        uint256 units,
        uint256 escrowAmount,
        uint32 blockNumberMod
    );

    /** @notice Called upon failed liquidity swap. */
    event SendLiquidityFailure(
        bytes32 channelId,
        bytes toAccount,
        uint256 units,
        uint256 escrowAmount,
        uint32 blockNumberMod
    );

    /** @notice Vault setup has been finalised. */
    event FinishSetup();

    /**
     * @notice Emitted on fee administrator adjustment
     * @param administrator The new vault fee administrator
     */
    event SetFeeAdministrator(
        address administrator
    );

    /**
     * @notice Emitted on vault fee adjustment
     * @param fee The new vault fee
     */
    event SetVaultFee(
        uint256 fee
    );

    /**
     * @notice Emitted on governance fee adjustment
     * @param fee The new governance fee
     */
    event SetGovernanceFee(
        uint256 fee
    );

    /**
     * @notice Emitted on weights modification
     * @param targetTime Time at which the weights adjustment must complete.
     * @param targetWeights The desired new weights.
     */
    event SetWeights(
        uint256 targetTime,
        uint256[] targetWeights
    );

    /**
     * @notice Amplification has been modification
     * @param targetTime Time at which the amplification adjustment must complete.
     * @param targetAmplification The desired new amplification.
     */
    event SetAmplification(
        uint256 targetTime,
        uint256 targetAmplification
    );

    /**
     * @notice A connection has been modified
     * @param channelId Target chain identifier.
     * @param toVault Bytes32 representation of the target vault.
     * @param newState Boolean indicating if the connection should be open or closed.
     */
    event SetConnection(
        bytes32 channelId,
        bytes toVault,
        bool newState
    );

    //-- Underwriting Events --//

    /**
     * @notice A swap has been underwritten.
     */
    event SwapUnderwritten(
        bytes32 indexed identifier,
        address toAsset,
        uint256 U,
        uint256 purchasedTokens
    );

}
