//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import "../interfaces/ICatalystV1PoolImmutables.sol";
import "./interfaces/ICatalystMathLib.sol";
import "../SwapPoolFactory.sol";

contract CatalystDescriber {

    uint256 internal _num_whitelisted_templates;
    mapping(uint256 => address) internal _whitelisted_templates;

    uint256 internal _num_whitelisted_ccis;
    mapping(uint256 => address) internal _whitelisted_ccis;


    uint256 internal _num_vault_factories;
    mapping(uint256 => address) internal _vault_factories;

    mapping(bytes32 => int256) internal _vault_abi_version;
    mapping(bytes32 => address) internal _vault_mathlib;  // TODO: Do we want to add this to vaults? It would simplify adding new contracts.


    // Returns an array of whitelisted vault templates.
    function get_whitelisted_templates() external view returns (address[] memory whitelistedTemplates) {
        for (uint256 it; it <= _num_whitelisted_templates; ++it) {
            address whitelisted_tempalte = _whitelisted_templates[it];
            whitelistedTemplates[it] = whitelisted_tempalte;
        }
    }


    // Returns an array of whitelisted CCIs.
    function get_whitelisted_CCI() external view returns (address[] memory whitelistedCCI) {
        for (uint256 it; it <= _num_whitelisted_ccis; ++it) {
            address whitelisted_cci = _whitelisted_ccis[it];
            whitelistedCCI[it] = whitelisted_cci;
        }
    }

    // Returns an array of vault factories.
    function get_vault_factories() external view returns (address[] memory poolFactories) {
        for (uint256 it; it <= _num_vault_factories; ++it) {
            address pool_factory = _vault_factories[it];
            poolFactories[it] = pool_factory;
        }
    }

    // Returns a pool’s factory.
    // This is fetched by asking the vault which factory deployed it, then checking with the factory.
    // Returns address(0) if the `address` lies.
    function get_vault_factory(address vault) external view returns (address factory) {
        address factory = ICatalystV1PoolImmutables(vault).FACTORY();
        address cci = ICatalystV1PoolImmutables(vault)._chainInterface();
        // Check if the factory agree
        if (!CatalystSwapPoolFactory(factory).IsCreatedByFactory(cci, vault)) factory = address(0);
    }

    // Returns the code hash of the address.
    function get_vault_type(address vault) public view returns (bytes32) {
        return vault.codehash;
    }


    // Returns the abi_version using a whitelisted table. Returns -1 if the address is not known
    function get_pool_abi_version(address) external view returns (int256 abi_version) {
        abi_version = _vault_abi_version[get_vault_type(vault)];
        if (abi_version == 0) abi_version = -1;
    }


    // Returns the pool tokens supported by a pool by iterating over _tokenIndexing until it returns 0
    function get_pool_tokens(address vault) public view returns (address[] memory vaultTokens) {
        for (uint256 it; true; ++it) {
            address token = ICatalystV1PoolImmutables(it)._tokenIndexing(it);
            if (token == address(0)) break;
            vaultTokens[it] = token;
        }
    }


    // Returns an address which implements and exposes the pool’s mathematical methods.
    // Uses get_pool_type to find a mathematical lib. Returns address(0) if no mathematical lib is set. (whitelist)
    function get_pool_mathematical_lib(address vault) public view returns (address math_lib) {
        math_lib = _vault_mathlib[vault];
    }


    // Returns a list of token prices. The first element is always a reference balance. (what is “1”)
    // Requires get_pool_mathematical_lib() ≠ address(0)
    function get_pool_prices(address vault) external view returns (uint256[] memory quotes) {
        address math_lib = get_pool_mathematical_lib(vault);
        if (math_lib == address(0)) return;
        address[] memory tokens = get_pool_tokens(vault);
        for (uint256 it; it < tokens.length; ++it) {
            address token = tokens[it];
            quotes[it] = CatalystMathLib(math_lib).calcAsyncPriceFrom(vault, token);
        }
    }
}