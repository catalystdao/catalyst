//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Events emitted by Catalyst v1 Factory
/// @notice Contains all events emitted by the Factory
interface ICatalystV1FactoryEvents {
    /**
     * @notice  Describes an atomic swap between the 2 tokens: _fromAsset and _toAsset.
     * @dev Should be used for vault discovery and pathing.
     * @param deployer msg.sender of the deploy function.
     * @param vault_address The minimal transparent proxy address for the swap vault.
     * @param chainInterface The address of the CCI used by the transparent proxy.
     * @param k Set to 10**18 if the vault is volatile, otherwise the vault is a stable vault.
     * @param assets List of the assets the vault supports.
     */
    event VaultDeployed(
        address indexed vaultTemplate,
        address indexed chainInterface, 
        address indexed deployer,
        address vault_address,
        address[] assets,
        uint256 k
    );

    /**
     * @notice Describes vault fee changes.
     * @dev Only applies to new vaults, has no impact on existing vaults.
     * @param fee The new vault fee.
     */
    event SetDefaultVaultFee(
        uint256 fee
    );

    /**
     * @notice Describes governance fee changes.
     * @dev Only applies to new vaults, has no impact on existing vaults.
     * @param fee The new governance fee.
     */
    event SetDefaultGovernanceFee(
        uint256 fee
    );

    /**
     * @notice Vault Template has been added.
     * @param vaultTemplateIndex The index of the vault template.
     * @param templateAddress The address of the vault template.
     */
    event AddVaultTemplate(
        uint256 vaultTemplateIndex,
        address templateAddress
    );
}
