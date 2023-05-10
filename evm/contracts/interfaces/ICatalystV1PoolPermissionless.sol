//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

interface ICatalystV1PoolPermissionless {
    /** 
     * @notice Setup a pool.
     * @param name_ Name of the Pool token.
     * @param symbol_ Symbol for the Pool token.
     * @param chainInterface The cross chain interface used for cross-chain swaps. (Can be address(0) to disable cross-chain swaps.)
     * @param poolFee The pool fee.
     * @param governanceFee The governance fee share.
     * @param feeAdministrator The account that can modify the fees.
     * @param setupMaster The short-term owner of the pool (until finishSetup is called).
     */
    function setup(
        string calldata name_,
        string calldata symbol_,
        address chainInterface,
        uint256 poolFee,
        uint256 governanceFee,
        address feeAdministrator,
        address setupMaster
    ) external;

    //--- Balance Changes ---//

    /**
     * @notice Deposits a user configurable amount of tokens.
     * @dev Requires approvals for all tokens within the pool.
     * Volatile: It is advised that the deposit matches the pool's %token distribution.
     * Amplified: It is advised that the deposit is as close to 1,1,... as possible.
     *            Otherwise between 1,1,... and the pool's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of pool tokens to be minted.
     */
    function depositMixed(uint256[] calldata tokenAmounts, uint256 minOut)
        external returns(uint256);

    /**
     * @notice Burns baseAmount and releases the symmetrical share
     * of tokens to the burner. This doesn't change the pool price.
     * @param baseAmount The number of pool tokens to burn.
     */
    function withdrawAll(uint256 baseAmount, uint256[] calldata minOut)
        external returns(uint256[] memory);

    /**
     * @notice Burns poolTokens and release a token distribution which can be set by the user.
     * @dev Requires approvals for all tokens within the pool.
     * Volatile: It is advised that the deposit matches the pool's %token distribution.
     * Amplified: It is advised that the deposit matches the pool's %token distribution.
     *            Otherwise it should be weighted towards the tokens the pool has more of.
     * @param poolTokens The number of pool tokens to withdraw.
     * @param withdrawRatio The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is X64
     * @param minOut The minimum number of tokens minted.
     */
    function withdrawMixed(
        uint256 poolTokens,
        uint256[] calldata withdrawRatio,
        uint256[] calldata minOut
    ) external returns(uint256[] memory);

    //--- Swaps ---//

    /**
     * @notice A swap between 2 assets which both are inside the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy.
     * @param amount The amount of fromAsset the user wants to sell.
     * @param minOut The minimum output of _toAsset the user wants.
     */
    function localSwap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) external returns (uint256);

    /**
     * @notice Initiate a cross-chain swap by purchasing units and transfer them to another pool.
     * @dev Addresses are encoded in 64 + 1 bytes. To encode for EVM, encode as:
     * Solidity: abi.encodePacket(uint8(20), bytes32(0), abi.encode(<poolAddress>))
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in 64 + 1 bytes.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in 64 + 1 bytes.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param amount The number of fromAsset to sell to the pool.
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @return uint256 The number of units minted.
     */
    function sendAsset(
        bytes32 channelId,
        bytes calldata toPool,
        bytes calldata toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256);

    /// @notice Includes calldata_
    /// @param calldata_ Data field if a call should be made on the target chain.
    /// Encoding depends on the target chain, with evm being: abi.encode(bytes20(<address>), <data>)
    function sendAsset(
        bytes32 channelId,
        bytes calldata toPool,
        bytes calldata toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        bytes memory calldata_
    ) external returns (uint256);

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token (toAsset)
     * @dev Can only be called by the chainInterface.
     * @param channelId The incoming connection identifier.
     * @param fromPool The source pool.
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
        bytes calldata fromPool,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod
    ) external returns (uint256);

    function receiveAsset(
        bytes32 channelId,
        bytes calldata fromPool,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        bytes calldata fromAsset,
        uint32 blockNumberMod,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256);

    /**
     * @notice Initiate a cross-chain liquidity swap by withdrawing tokens and converting them to units.
     * @dev While the description says tokens are withdrawn and then converted to units, pool tokens are converted
     * directly into units through the following equation:
     *      U = ln(PT/(PT-pt)) * \sum W_i
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in 64 + 1 bytes.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in 64 bytes + 1.
     * @param poolTokens The number of pool tokens to exchange.
     * @param minOut An array of minout describing: [the minimum number of pool tokens, the minimum number of reference assets].
     * @param fallbackUser If the transaction fails, send the escrowed funds to this address.
     * @return uint256 The number of units minted.
     */
    function sendLiquidity(
        bytes32 channelId,
        bytes calldata toPool,
        bytes calldata toAccount,
        uint256 poolTokens,
        uint256[2] calldata minOut,
        address fallbackUser
    ) external returns (uint256);

    /// @notice Includes calldata_
    /// @param calldata_ Data field if a call should be made on the target chain.
    /// Encoding depends on the target chain, with evm being: abi.encode(bytes20(<address>), <data>)
    function sendLiquidity(
        bytes32 channelId,
        bytes calldata toPool,
        bytes calldata toAccount,
        uint256 poolTokens,
        uint256[2] calldata minOut,
        address fallbackUser,
        bytes memory calldata_
    ) external returns (uint256);

    /**
     * @notice Completes a cross-chain liquidity swap by converting units to tokens and depositing.
     * @dev Called exclusively by the chainInterface.
     * @param fromPool The source pool
     * @param toAccount The recipient of the pool tokens
     * @param U Number of units to convert into pool tokens.
     * @param minPoolTokens The minimum number of pool tokens to mint on target pool. Otherwise: Reject
     * @param minReferenceAsset The minimum number of reference asset the pools tokens are worth. Otherwise: Reject
     * @param fromAmount Used to connect swaps cross-chain. The input amount on the sending chain.
     * @param blockNumberMod Used to connect swaps cross-chain. The block number from the host side.
     * @return uint256 Number of pool tokens minted to the recipient.
     */
    function receiveLiquidity(
        bytes32 channelId,
        bytes calldata fromPool,
        address toAccount,
        uint256 U,
        uint256 minPoolTokens,
        uint256 minReferenceAsset,
        uint256 fromAmount,
        uint32 blockNumberMod
    ) external returns (uint256);

    function receiveLiquidity(
        bytes32 channelId,
        bytes calldata fromPool,
        address who,
        uint256 U,
        uint256 minPoolTokens,
        uint256 minReferenceAsset,
        uint256 fromAmount,
        uint32 blockNumberMod,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256);
}
