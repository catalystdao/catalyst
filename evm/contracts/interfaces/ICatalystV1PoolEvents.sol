//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Events emitted by Catalyst v1 Pools
/// @notice Contains all events emitted by the pool
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
     * @param toPool The target pool.
     * @param targetUser The recipient of the trade. The person who bought the trade is not present.
     * @param fromAsset The asset which was sold in exchange for _toAsset.
     * @param toAssetIndex The token index of the asset to purchase on _toChain.
     * @param fromAmount The number of _fromAsset sold
     * @param units The calculated number of units bought. Will be sold to buy _toAsset
     * @param minOut The pool fee. Taken from fromAmount. Numerical losses/fees are for obvious reasons not included.
     */
    event SendSwap(
        bytes32 indexed toPool,
        bytes32 indexed targetUser,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 fromAmount,
        uint256 units,
        uint256 minOut,
        bytes32 swapHash
    );

    /**
     * @notice Describes the arrival of an external swap: Cross-chain swap.
     * @dev If _fromAsset is the proxy contract, the swap is a liquidity swap.
     * @param sourcePool The source pool.
     * @param toAccount The recipient of the trade.
     * @param toAsset The asset which was purchased with _fromAsset
     * @param units The number of units sent from the other chain.
     * @param toAmount The number of tokens provided to toAccount
     */
    event ReceiveSwap(
        bytes32 sourcePool,
        address indexed toAccount,
        address toAsset,
        uint256 units,
        uint256 toAmount,
        bytes32 swapHash
    );

    /**
     * @notice Describes the creation of a liquidity swap
     * @param toPool The target pool.
     * @param targetUser The recipient of the liquidity. The person who bought the trade is not present.
     * @param fromAmount The number of _fromAsset sold
     * @param units The calculated number of liquidity units bought.
     */
    event SendLiquidity(
        bytes32 indexed toPool,
        bytes32 indexed targetUser,
        uint256 fromAmount,
        uint256 units,
        bytes32 swapHash
    );

    /**
     * @notice Describes the arrival of a liquidity swap
     * @param sourcePool The source pool.
     * @param toAccount The recipient of the liquidity.
     * @param units The number of liquidity units sent from the other chain.
     * @param toAmount The number of pool tokens provided to toAccount
     */
    event ReceiveLiquidity(
        bytes32 sourcePool,
        address indexed toAccount,
        uint256 units,
        uint256 toAmount,
        bytes32 swapHash
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

    /** @notice Called upon successful swap. */
    event EscrowAck(bytes32 swapHash, bool liquiditySwap);

    /** @notice Called upon failed swap. */
    event EscrowTimeout(bytes32 swapHash, bool liquiditySwap);

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
    event ModifyWeights(
        uint256 targetTime,
        uint256[] targetWeights
    );

    /**
     * @notice Amplification has been modification
     * @param targetTime Time at which the amplification adjustment must complete.
     * @param targetAmplification The desired new amplification.
     */
    event ModifyAmplification(
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
        bytes32 toPool,
        bool newState
    );
}
