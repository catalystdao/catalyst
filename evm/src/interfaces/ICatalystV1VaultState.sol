//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Vault state
/// @notice Contains all vault storage which depends on the vault state.
interface ICatalystV1VaultState {
    /// @notice If the vault has no cross chain connection, this is true. Should not be trusted if setupMaster != ZERO_ADDRESS
    function onlyLocal() external view returns (bool);

    function _vaultConnection(bytes32 channelIdentifier, bytes calldata remoteVault) external view returns (bool);

    /// @notice The token weights. Used for maintaining a non symmetric vault asset balance.
    function _weight(address token) external view returns (uint256);

    function _adjustmentTarget() external view returns (uint256);

    function _lastModificationTime() external view returns (uint256);

    /// @notice The vault fee in WAD. Implementation of fee: mulWadDown(_amount, _vaultFee)
    function _vaultFee() external view returns (uint256);

    function _governanceFeeShare() external view returns (uint256);

    /// @notice The address of the responsible for adjusting the fees.
    function _feeAdministrator() external view returns (address);

    /// @notice The setupMaster is the short-term owner of the vault. They can connect the vault to vaults on other chains.
    function _setupMaster() external view returns (address);

    // Security limit
    /// @notice The max incoming liquidity flow from the router.
    function _maxUnitCapacity() external view returns (uint256);

    // Escrow reference
    /// @notice Total current escrowed tokens
    function _escrowedTokens(address token) external view returns (uint256);

    /// @notice Find escrow information. Used for both normal swaps and liquidity swaps.
    function _escrowLookup(bytes32 sendAssetHash) external view returns (address);

    /// @notice Total current escrowed vault tokens
    function _escrowedVaultTokens() external view returns (uint256);

    function factoryOwner() external view returns (address);

    /**
     * @notice External view function purely used to signal if a vault is safe to use.
     * @dev Just checks if the setup master has been set to ZERO_ADDRESS. In other words, has finishSetup been called?
     */
    function ready() external view returns (bool);
}
