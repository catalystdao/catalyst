//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Administrative actions defined by Catalyst v1 Vaults
/// @notice Contains all functions which can only be called by privileged users.
interface ICatalystV1VaultAdministration {
    function setFeeAdministrator(address administrator) external;

    function setVaultFee(uint256 fee) external;

    function setGovernanceFee(uint256 fee) external;

    /**
     * @notice Initializes the vault pricing parameters.
     * @param assets The list of assets the vault will support.
     * @param weights The weights of the tokens.
     * @param amp Vault amplification.
     * @param depositor The account to which the initial vault tokens are minted to.
     */
    function initializeSwapCurves(
        address[] calldata assets,
        uint256[] calldata weights,
        uint256 amp,
        address depositor
    ) external;

    /**
     * @notice Creates a connection to the vault toVault on the channel _channelId.
     * @dev if _vaultReceiving is an EVM vault, it can be computes as:
     *     Vyper: convert(_vaultAddress, bytes32)
     *     Solidity: abi.encode(_vaultAddress)
     *     Brownie: brownie.convert.to_bytes(_vaultAddress, type_str="bytes32")
     * setupMaster == ZERO_ADDRESS
     * @param channelId The _channelId of the target vault.
     * @param toVault The bytes32 representation of the target vault
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function setConnection(
        bytes32 channelId,
        bytes calldata toVault,
        bool state
    ) external;

    /**
     * @notice Gives up short term ownership of the vault. This makes the vault unstoppable.
     */
    function finishSetup() external;
}
