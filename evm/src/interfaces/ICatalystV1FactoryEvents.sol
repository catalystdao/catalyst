//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Events emitted by Catalyst v1 Factory
/// @notice Contains all events emitted by the Factory
interface ICatalystV1FactoryEvents {
    /**
     * @notice  Describes the deployment of a new vault as a proxy of the given vault template.
     * @dev Should be used for vault discovery and pathing.
     * @param vaultTemplate The address of the template used by the transparent proxy.
     * @param chainInterface The address of the CCI used by the transparent proxy.
     * @param vaultAddress The minimal transparent proxy address for the vault.
     * @param assets List of the assets the vault supports.
     * @param k Set to 10**18 if the vault is volatile, otherwise the vault is an amplified vault.
     */
    event VaultDeployed(
        address indexed vaultTemplate,
        address indexed chainInterface, 
        address indexed deployer,
        address vaultAddress,
        address[] assets,
        uint256 k
    );

    /**
     * @notice Describes governance fee changes.
     * @dev Only applies to new vaults, has no impact on existing vaults.
     * @param fee The new governance fee.
     */
    event SetDefaultGovernanceFee(
        uint256 fee
    );
}
