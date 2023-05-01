//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Events emitted by Catalyst v1 Pools
/// @notice Contains all events emitted by the pool
/// @dev When using events to match transations, the combination of: channelId, fromPool, toAccount, toAsset, units, and block number is semi-guranteed to be unique.
///     If more than 2**32 blocks exist, then all instances are guaranteed to be non-overlapping
interface ICatalystV1PoolEvents {
    /**
     * @notice  Describes an atomic swap between the 2 tokens: _fromAsset and _toAsset.
     * @dev Explain to a developer any extra details
     * @param toAccount The user / exchange who facilitated the trade (msg.sender)
     * @param fromAsset The asset which was sold in exchange for _toAsset
     * @param toAsset The asset which was purchased with _fromAsset
     * @param fromAmount The number of _fromAsset sold
     * @param toAmount The number of tokens provided to toAccount
     */
    event LocalSwap(
        address indexed toAccount,
        address fromAsset,
        address toAsset,
        uint256 fromAmount,
        uint256 toAmount
    );

    /**
     * @notice Describes the creation of an external swap: Cross-chain swap.
     * @dev If _fromAsset is the proxy contract or _toAsset is 2**8-1, the swap is a liquidity swap.
     * @param channelId The target chain identifier
     * @param toPool The target pool.
     * @param toAccount The recipient of the trade. The person who bought the trade is not present.
     * @param fromAsset The asset which was sold in exchange for _toAsset.
     * @param toAssetIndex The token index of the asset to purchase on _toChain.
     * @param fromAmount The number of _fromAsset sold.
     * @param minOut The pool fee. Taken from fromAmount. Numerical losses/fees are for obvious reasons not included.
     * @param units The calculated number of units bought. Will be sold to buy _toAsset
     * @param fee The number of tokens paid to the pool in fees.
     */
    event SendAsset(
        bytes32 channelId,
        bytes toPool,
        bytes toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 fromAmount,
        uint256 minOut,
        uint256 units,
        uint256 fee
    );

    /**
     * @notice Describes the arrival of an external swap: Cross-chain swap.
     * @dev If _fromAsset is the proxy contract, the swap is a liquidity swap.
     * If toAccount is used to match trades, remember to convert it into 64 + 1 bytes.
     * @param channelId The target chain identifier
     * @param fromPool The source pool.
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
        bytes fromPool,
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
     * @param toPool The target pool.
     * @param toAccount The recipient of the liquidity. The person who bought the trade is not present.
     * @param fromAmount The number of _fromAsset sold
     * @param minOut An array containing a list of minimum outputs [minPoolTokens, minReferenceAssets]
     * @param units The calculated number of liquidity units bought.
     */
    event SendLiquidity(
        bytes32 channelId,
        bytes toPool,
        bytes toAccount,
        uint256 fromAmount,
        uint256[2] minOut,
        uint256 units
    );

    /**
     * @notice Describes the arrival of a liquidity swap
     * @param channelId The target chain identifier
     * @param fromPool The source pool.
     * @param toAccount The recipient of the liquidity.
     * @param units The number of liquidity units sent from the other chain.
     * @param toAmount The number of pool tokens provided to toAccount
     * @param fromAmount The amount spent to get units on the source side.
     * @param sourceBlockNumberMod The block number of the sending transaction mod 2**32 - 1
     */
    event ReceiveLiquidity(
        bytes32 channelId,
        bytes fromPool,
        address toAccount,
        uint256 units,
        uint256 toAmount,
        uint256 fromAmount,
        uint256 sourceBlockNumberMod
    );

    /**
     * @notice Emitted on liquidity deposits.
     * @dev Explain to a developer any extra details
     * @param toAccount The depositor. Is credited with _mints pool tokens.
     * @param mint The number of minted pool tokens credited to toAccount
     * @param assets An array of the number of deposited assets.
     */
    event Deposit(address indexed toAccount, uint256 mint, uint256[] assets);

    /**
     * @notice Emitted on liquidity withdrawal.
     * @dev Explain to a developer any extra details
     * @param toAccount The withdrawer. Is debited _burns pool tokens.
     * @param burn The number of burned pool tokens.
     * @param assets An array of the token amounts returned
     */
    event Withdraw(address indexed toAccount, uint256 burn, uint256[] assets);

    /** @notice Called upon successful asset swap. */
    event SendAssetAck(
        bytes toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    );

    /** @notice Called upon failed asset swap. */
    event SendAssetTimeout(
        bytes toAccount,
        uint256 U,
        uint256 escrowAmount,
        address escrowToken,
        uint32 blockNumberMod
    );

    /** @notice Called upon successful liquidity swap. */
    event SendLiquidityAck(
        bytes toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
        );

    /** @notice Called upon failed liquidity swap. */
    event SendLiquidityTimeout(
        bytes toAccount,
        uint256 U,
        uint256 escrowAmount,
        uint32 blockNumberMod
    );

    /** @notice Pool setup has been finalised. */
    event FinishSetup();

    /**
     * @notice Emitted on fee administrator adjustment
     * @param administrator The new pool fee administrator
     */
    event SetFeeAdministrator(
        address administrator
    );

    /**
     * @notice Emitted on pool fee adjustment
     * @param fee The new pool fee
     */
    event SetPoolFee(
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
     * @param toPool Bytes32 representation of the target pool.
     * @param newState Boolean indicating if the connection should be open or closed.
     */
    event SetConnection(
        bytes32 channelId,
        bytes toPool,
        bool newState
    );
}
