//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Events emitted by Catalyst v1 Factory
/// @notice Contains all events emitted by the Factory
interface ICatalystV1FactoryEvents {
    /**
     * @notice  Describes an atomic swap between the 2 tokens: _fromAsset and _toAsset.
     * @dev Should be used for pool discovery and pathing.
     * @param deployer msg.sender of the deploy function.
     * @param pool_address The minimal transparent proxy address for the swap pool.
     * @param chainInterface The address of the CCI used by the transparent proxy.
     * @param k Set to 10**18 if the pool is volatile, otherwise the pool is a stable pool.
     * @param assets List of the assets the pool supports.
     */
    event PoolDeployed(
        address indexed deployer,
        address indexed pool_address,
        address indexed chainInterface, 
        address[] assets,
        address poolTemplate,
        uint256 k
    );

    /**
     * @notice Describes pool fee changes.
     * @dev Only applies to new pools, has no impact on existing pools.
     * @param fee The new pool fee.
     */
    event SetDefaultPoolFee(
        uint256 fee
    );

    /**
     * @notice Describes governance fee changes.
     * @dev Only applies to new pools, has no impact on existing pools.
     * @param fee The new governance fee.
     */
    event SetDefaultGovernanceFee(
        uint256 fee
    );

    /**
     * @notice Pool Template has been added.
     * @param poolTemplateIndex The index of the pool template.
     * @param templateAddress The address of the pool template.
     */
    event AddPoolTemplate(
        uint256 poolTemplateIndex,
        address templateAddress
    );
}
