//SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.16;

interface ICatalystV1PoolPermissionless {
    /** @notice Setup a pool. */
    function setup(
        string calldata name_,
        string calldata symbol_,
        address chainInterface,
        uint256 poolFee,
        uint256 governanceFee,
        address feeAdministrator,
        address setupMaster
    ) external;

    /** @notice Initialize the swap curves of the pool. */
    function initializeSwapCurves(
        address[] calldata assets,
        uint256[] calldata weights,
        uint256 amp,
        address depositor
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
     * @param poolTokens The number of pool tokens to withdraw
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
     * @param toAsset The asset the user wants to buy
     * @param amount The amount of fromAsset the user wants to sell
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
     * @dev Encoding addresses in bytes32 can be done be computed with:
     * Vyper: convert(<poolAddress>, bytes32)
     * Solidity: abi.encode(<poolAddress>)
     * Brownie: brownie.convert.to_bytes(<poolAddress>, type_str="bytes32")
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in bytes32.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in bytes32.
     * @param fromAsset The asset the user wants to sell.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param amount The number of fromAsset to sell to the pool.
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param fallbackUser If the transaction fails send the escrowed funds to this address
     */
    function sendAsset(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256);

    /// @notice Includes calldata_
    /// @param calldata_ Data field if a call should be made on the target chain. 
    /// Should be encoded abi.encode(<address>,<data>)
    function sendAsset(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        address fromAsset,
        uint8 toAssetIndex,
        uint256 amount,
        uint256 minOut,
        address fallbackUser,
        bytes memory calldata_
    ) external returns (uint256);

    /**
     * @notice Completes a cross-chain swap by converting units to the desired token (toAsset)
     *  Called exclusively by the chainInterface.
     * @dev Can only be called by the chainInterface, as there is no way to check the validity of units.
     * @param channelId The incoming connection identifier.
     * @param fromPool The source pool.
     * @param toAssetIndex Index of the asset to be purchased with _U units.
     * @param toAccount The recipient of toAsset
     * @param U Number of units to convert into toAsset.
     * @param minOut Minimum number of tokens bought. Reverts if less.
     */
    function receiveAsset(
        bytes32 channelId,
        bytes32 fromPool,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash
    ) external returns (uint256);

    function receiveAsset(
        bytes32 channelId,
        bytes32 fromPool,
        uint256 toAssetIndex,
        address toAccount,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256);

    /**
     * @notice Initiate a cross-chain liquidity swap by lowering liquidity
     * and transfer the liquidity units to another pool.
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in bytes32. For EVM chains this can be computed as:
     * Vyper: convert(_poolAddress, bytes32)
     * Solidity: abi.encode(_poolAddress)
     * Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * @param toAccount The recipient of the transaction on _chain. Encoded in bytes32. For EVM chains it can be found similarly to _targetPool.
     * @param baseAmount The number of pool tokens to liquidity Swap
     */
    function sendLiquidity(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        uint256 baseAmount,
        uint256 minOut,
        address fallbackUser
    ) external returns (uint256);

    /// @notice Includes calldata_
    /// @param calldata_ Data field if a call should be made on the target chain. 
    /// Should be encoded abi.encode(<address>,<data>)
    function sendLiquidity(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 who,
        uint256 baseAmount,
        uint256 minOut,
        address fallbackUser,
        bytes memory calldata_
    ) external returns (uint256);

    /**
     * @notice Completes a cross-chain swap by converting liquidity units to pool tokens
     * Called exclusively by the chainInterface.
     * @dev Can only be called by the chainInterface, as there is no way
     * to check the validity of units.
     * @param channelId The incoming connection identifier.
     * @param fromPool The source pool
     * @param toAccount The recipient of pool tokens
     * @param U Number of units to convert into pool tokens.
     */
    function receiveLiquidity(
        bytes32 channelId,
        bytes32 fromPool,
        address toAccount,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash
    ) external returns (uint256);

    function receiveLiquidity(
        bytes32 channelId,
        bytes32 fromPool,
        address who,
        uint256 U,
        uint256 minOut,
        bytes32 swapHash,
        address dataTarget,
        bytes calldata data
    ) external returns (uint256);
}
