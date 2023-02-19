//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

// Hold swap details that are not directly necessary for the swap calculations in a separate struct.
// (Avoid 'stack too deep' issues)
struct AssetSwapMetadata {
    uint256 fromAmount;
    address fromAsset;
    bytes32 swapHash;
    uint32  blockNumber;
}

struct LiquiditySwapMetadata {
    uint256 fromAmount;
    bytes32 swapHash;
    uint32  blockNumber;
}

/// @title Pool state
/// @notice Contains all pool storage which depends on the pool state.
interface ICatalystV1PoolState {
    /// @notice
    ///     If the pool has no cross chain connection, this is true.
    ///     Should not be trusted if setupMaster != ZERO_ADDRESS
    function onlyLocal() external view returns (bool);

    /// @notice The token weights. Used for maintaining a non symmetric pool balance.
    function _weight(address token) external view returns (uint256);

    function _adjustmentTarget() external view returns (uint256);

    function _lastModificationTime() external view returns (uint256);

    /// @notice The pool fee in X64. Implementation of fee: mulX64(_amount, self.poolFeeX64)
    function _poolFee() external view returns (uint256);

    function _governanceFeeShare() external view returns (uint256);

    /// @notice The address of the responsible for adjusting the fees.
    function _feeAdministrator() external view returns (address);

    /// @notice The setupMaster is the short-term owner of the pool.
    ///     They can connect the pool to pools on other chains.
    function _setupMaster() external view returns (address);

    //--- Messaging router limit ---//
    // The router is not completely trusted. Some limits are
    // imposed on the DECAY_RATE-ly unidirectional liquidity flow. That is:
    // if the pool observes more than self.maxUnitCapacity of incoming
    // units, then it will not accept further volume. This means the router
    // can only drain a prefigured percentage of the pool every DECAY_RATE

    // Outgoing flow is subtracted incoming flow until 0.

    /// @notice The max incoming liquidity flow from the router.
    function _maxUnitCapacity() external view returns (uint256);

    // uint256 public max_liquidity_unit_inflow = totalSupply / 2

    // Escrow reference
    /// @notice Total current escrowed tokens
    function _escrowedTokens(address token) external view returns (uint256);

    /// @notice Specific escrow information
    // function _escrowedFor(bytes32 sendAssetHash) external view returns (TokenEscrow calldata);

    /// @notice Total current escrowed pool tokens
    function _escrowedPoolTokens() external view returns (uint256);

    /// @notice Specific escrow information (Pool Tokens)
    // function _escrowedLiquidityFor(bytes32 sendLiquidityHash) external view returns (LiquidityEscrow memory);

    function factoryOwner() external view returns (address);

    /**
     * @notice
     *     External view function purely used to signal if a pool is safe to use.
     * @dev
     *     Just checks if the setup master has been set to ZERO_ADDRESS.
     *     In other words, has finishSetup been called?
     */
    function ready() external view returns (bool);
}
