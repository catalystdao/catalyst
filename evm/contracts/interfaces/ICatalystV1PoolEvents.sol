//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Events emitted by Catalyst v1 Pools
/// @notice Contains all events emitted by the pool
interface ICatalystV1PoolEvents {
    /**
     * @notice  Describes an atomic swap between the 2 tokens: _fromAsset and _toAsset.
     * @dev Explain to a developer any extra details
     * @param who The user / exchange who facilitated the trade (msg.sender)
     * @param fromAsset The asset which was sold in exchange for _toAsset
     * @param toAsset The asset which was purchased with _fromAsset
     * @param input The number of _fromAsset sold
     * @param output The number of tokens provided to _who
     */
    event LocalSwap(
        address indexed who,
        address indexed fromAsset,
        address toAsset,
        uint256 input,
        uint256 output
    );

    /**
     * @notice Describes the creation of an external swap: Cross-chain swap.
     * @dev If _fromAsset is the proxy contract or _toAsset is 2**8-1, the swap is a liquidity swap.
     * @param targetPool The target pool.
     * @param targetUser The recipient of the trade. The person who bought the trade is not present.
     * @param fromAsset The asset which was sold in exchange for _toAsset.
     * @param toAssetIndex The token index of the asset to purchase on _toChain.
     * @param input The number of _fromAsset sold
     * @param output The calculated number of units bought. Will be sold to buy _toAsset
     * @param minOut The pool fee. Taken from the input. Numerical losses/fees are for obvious reasons not included.
     */
    event SendSwap(
        bytes32 indexed targetPool,
        bytes32 indexed targetUser,
        address indexed fromAsset,
        uint8 toAssetIndex,
        uint256 input,
        uint256 output,
        uint256 minOut,
        bytes32 messageHash
    );

    /**
     * @notice Describes the arrival of an external swap: Cross-chain swap.
     * @dev If _fromAsset is the proxy contract, the swap is a liquidity swap.
     * @param who The recipient of the trade.
     * @param toAsset The asset which was purchased with _fromAsset
     * @param input The number of units sent from the other chain.
     * @param output The number of tokens provided to _who
     */
    event ReceiveSwap(
        address indexed who,
        address indexed toAsset,
        uint256 input,
        uint256 output,
        bytes32 messageHash
    );

    /**
     * @notice Describes the creation of a liquidity swap
     * @param targetPool The target pool.
     * @param targetUser The recipient of the liquidity. The person who bought the trade is not present.
     * @param input The number of _fromAsset sold
     * @param output The calculated number of liquidity units bought.
     */
    event SendLiquidity(
        bytes32 indexed targetPool,
        bytes32 indexed targetUser,
        uint256 input,
        uint256 output,
        bytes32 messageHash
    );

    /**
     * @notice Describes the arrival of a liquidity swap
     * @param who The recipient of the liquidity.
     * @param input The number of liquidity units sent from the other chain.
     * @param output The number of pool tokens provided to _who
     */
    event ReceiveLiquidity(
        address indexed who,
        uint256 input,
        uint256 output,
        bytes32 messageHash
    );

    /**
     * @notice Emitted on liquidity deposits.
     * @dev Explain to a developer any extra details
     * @param who The depositor. Is credited with _mints pool tokens.
     * @param mint The number of minted pool tokens credited to _who
     * @param assets An array of the number of deposited assets.
     */
    event Deposit(address indexed who, uint256 mint, uint256[] assets);

    /**
     * @notice Emitted on liquidity withdrawal.
     * @dev Explain to a developer any extra details
     * @param who The withdrawer. Is debited _burns pool tokens.
     * @param burn The number of burned pool tokens.
     * @param assets An array of the token amounts returned
     */
    event Withdraw(address indexed who, uint256 burn, uint256[] assets);

    /** @notice Called upon successful swap. */
    event EscrowAck(bytes32 messageHash, bool liquiditySwap);

    /** @notice Called upon failed swap. */
    event EscrowTimeout(bytes32 messageHash, bool liquiditySwap);


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
     * @param targetPool Bytes32 representation of the target pool.
     * @param newState Boolean indicating if the connection should be open or closed.
     */
    event SetConnection(
        bytes32 channelId,
        bytes32 targetPool,
        bool newState
    );
}
