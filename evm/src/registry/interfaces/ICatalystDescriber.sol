//SPDX-License-Identifier: MIT

pragma solidity ^0.8.17;

interface ICatalystDescriber {

    // Returns an array of whitelisted vault templates.
    function get_whitelisted_templates() external view returns (address[] memory whitelistedTemplates);

    // Returns an array of whitelisted CCIs.
    function get_whitelisted_CCI() external view returns (address[] memory whitelistedCCI);

    // Returns an array of vault factories.
    function get_vault_factories() external view returns (address[] memory vaultFactories);

    // Returns a vault’s factory.
    // This is fetched by asking the vault which factory deployed it, then checking with the factory.
    // Returns address(0) if the `address` lies.
    function get_vault_factory(address vault) external view returns (address factory);

    // Returns the code hash of the address.
    function get_vault_type(address vault) external view returns (bytes32);

    // Returns the abi_version using a whitelisted table. Returns -1 if the address is not known
    function get_vault_abi_version(address) external view returns (int256 abi_version);

    // Returns the vault tokens supported by a vault by iterating over _tokenIndexing until it returns 0
    function get_vault_tokens(address vault) external view returns (address[] memory vaultTokens);

    // Returns an address which implements and exposes the vault’s mathematical methods.
    // Uses get_vault_type to find a mathematical lib. Returns address(0) if no mathematical lib is set. (whitelist)
    function get_vault_mathematical_lib(address) external view returns (address);

    // Returns a list of token prices. The first element is always a reference balance. (what is “1”)
    // Requires get_vault_mathematical_lib() ≠ address(0)
    function get_vault_prices(address) external view returns (uint256[] memory);
}