//SPDX-License-Identifier: Unlicsened

pragma solidity ^0.8.17;

interface ICatalystV1PoolPermissionless {
    /** @notice Setup a pool. */
    function setup(
        address[] calldata init_assets,
        uint256[] calldata weights,
        uint256 amp,
        uint256 governanceFee,
        string calldata name_,
        string calldata symbol_,
        address chaininterface,
        address setupMaster
    ) external;

    /**
     * @notice Deposits a user configurable amount of tokens.
     * @dev Requires approvals for all tokens within the pool.
     * Volatile: It is advised that the deposit matches the pool's %token distribution.
     * Amplified: It is advised that the deposit is as close to 1,1,... as possible. 
     *            Otherwise between 1,1,... and the pool's %token distribution.
     * @param tokenAmounts An array of the tokens amounts to be deposited.
     * @param minOut The minimum number of pool tokens to be minted.
     */
    function depositMixed(uint256[] calldata tokenAmounts, uint256 minOut) external;

    /**
     * @notice Burns baseAmount and releases the symmetrical share
     * of tokens to the burner. This doesn't change the pool price.
     * @param baseAmount The number of pool tokens to burn.
     */
    function withdrawAll(uint256 baseAmount, uint256[] calldata minOut) external;

    /**
     * @notice Burns poolTokens and release a token distribution which can be set by the user.
     * @dev Requires approvals for all tokens within the pool.
     * Volatile: It is advised that the deposit matches the pool's %token distribution.
     * Amplified: It is advised that the deposit matches the pool's %token distribution.
     *            Otherwise it should be weighted towards the tokens the pool has more of.
     * @param poolTokens The number of pool tokens to withdraw
     * @param withdrawRatioX64 The percentage of units used to withdraw. In the following special scheme: U_a = U · withdrawRatio[0], U_b = (U - U_a) · withdrawRatio[1], U_c = (U - U_a - U_b) · withdrawRatio[2], .... Is X64
     * @param minOuts The minimum number of tokens minted.
     */
    function withdrawMixed(uint256 poolTokens, uint256[] calldata withdrawRatioX64, uint256[] calldata minOuts) external;

    /**
     * @notice A swap between 2 assets which both are inside the pool. Is atomic.
     * @param fromAsset The asset the user wants to sell.
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
     * @param minOut The minimum output of _toAsset the user wants.
     */
    function localswap(
        address fromAsset,
        address toAsset,
        uint256 amount,
        uint256 minOut
    ) external returns (uint256);

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
        address fallbackUser
    ) external returns (uint256);

    /// @notice Includes calldata_
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
        bytes calldata calldata_
    ) external returns (uint256);

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token (toAsset)
     *  Called exclusively by the chaininterface.
     * @dev Can only be called by the chaininterface, as there is no way to check the validity of units.
     * @param toAssetIndex Index of the asset to be purchased with _U units.
     * @param who The recipient of toAsset
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     * @param approx If the swap approximation should be used over the "true" swap. Ignored for amplified pools.
     */
    function swapFromUnits(
        uint256 toAssetIndex,
        address who,
        uint256 U,
        uint256 minOut,
        bool approx,
        bytes32 messageHash
    ) external returns (uint256);

    function swapFromUnits(
        uint256 toAssetIndex,
        address who,
        uint256 U,
        uint256 minOut,
        bool approx,
        bytes32 messageHash,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256);

    /**
     * @notice Initiate a cross-chain liquidity swap by lowering liquidity
     * and transfer the liquidity units to another pool.
     * @param chain The target chain. Will be converted by the interface to channelId.
     * @param targetPool The target pool on the target chain encoded in bytes32. For EVM chains this can be computed as:
     * Vyper: convert(_poolAddress, bytes32)
     * Solidity: abi.encode(_poolAddress)
     * Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * @param who The recipient of the transaction on _chain. Encoded in bytes32. For EVM chains it can be found similarly to _targetPool.
     * @param baseAmount The number of pool tokens to liquidity Swap
     */
    function outLiquidity(
        uint256 chain,
        bytes32 targetPool,
        bytes32 who,
        uint256 baseAmount,
        uint256 minOut,
        uint8 approx,
        address fallbackUser
    ) external returns (uint256);

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
    ) external returns (uint256);
}
