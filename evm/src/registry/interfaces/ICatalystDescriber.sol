//SPDX-License-Identifier: MIT
pragma solidity ^0.8.17;

interface ICatalystDescriber {

    /** @notice Returns an array of whitelisted vault templates. */
    function getWhitelistedTemplates() external view returns (address[] memory whitelistedTemplates);

    /** @notice Returns an array of whitelisted CCIs. */
    function getWhitelistedCCI() external view returns (address[] memory whitelistedCCI);

    /** @notice Returns an array of vault factories. */
    function getVaultFactories() external view returns (address[] memory vaultFactories);

    /**
     * @notice  Returns a vault's factory.
     * This is fetched by asking the vault which factory deployed it, then checking with the factory.
     * Returns address(0) if the `address` lies.
     */
    function getFactoryOfVault(address vault) external view returns (address factory);

    /** @notice Returns the code hash of the address. */
    function getVaultType(address vault) external view returns (bytes32);

    /** @notice Returns the abi_version using a whitelisted table. Returns -1 if the address is not known */
    function getVaultABIVersion(address) external view returns (int256 abi_version);

    /** @notice Returns the vault tokens supported by a vault by iterating over _tokenIndexing until it returns 0 */
    function getVaultTokens(address vault) external view returns (address[] memory vaultTokens);

    /**
     * @notice Returns an address which implements and exposes the vault’s mathematical methods.
     * Uses getVaultType to find a mathematical lib. Returns address(0) if no mathematical lib is set. (whitelist)
     */
    function getVaultMathematicalLib(address) external view returns (address);

    /**
     * Returns a list of token prices. The first element is always a reference balance. (what is “1”)
     * Requires getVaultMathematicalLib() ≠ address(0)
     */
    function getVaultPrices(address) external view returns (uint256[] memory);
}