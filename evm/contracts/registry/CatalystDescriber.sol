//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import "../interfaces/ICatalystV1PoolImmutables.sol";
import "./interfaces/ICatalystMathLibCommon.sol";
import "../SwapPoolFactory.sol";

contract CatalystDescriber is Ownable {
    error ZeroAddress();
    error InvalidIndex(address providedAddress, address readAddress);
    error IncorrectAbi();

    event ModifyWhitelistedTemplate(
        address template,
        bool state
    );

    event ModifyVaultAbi(
        address template,
        int256 abi_version
    );

    event ModifyWhitelistedCCI(
        address cci,
        bool state
    );

    event ModifyPoolFactory(
        address pool_factory,
        bool state
    );

    address[] private _whitelisted_templates;

    address[] private _whitelisted_ccis;

    address[] private _vault_factories;

    mapping(bytes32 => int256) internal _vault_abi_version;


    // Returns an array of whitelisted vault templates.
    function get_whitelisted_templates() external view returns (address[] memory whitelistedTemplates) {
        return _whitelisted_templates;
    }

    function get_num_whitelisted_templates() external view returns(uint256) {
        return _whitelisted_templates.length;
    } 


    // Returns an array of whitelisted CCIs.
    function get_whitelisted_CCI() external view returns (address[] memory whitelistedCCI) {
        return _whitelisted_ccis;
    }

    function get_num_whitelisted_ccis() external view returns(uint256) {
        return _whitelisted_ccis.length;
    } 

    // Returns an array of vault factories.
    function get_vault_factories() external view returns (address[] memory poolFactories) {
        return _vault_factories;
    }

    // Returns an array of vault factories.
    function get_num_vault_factories() external view returns (uint256) {
        return _vault_factories.length;
    }

    // Returns a pool’s factory.
    // This is fetched by asking the vault which factory deployed it, then checking with the factory.
    // Returns address(0) if the `address` lies.
    function get_vault_factory(address vault) external view returns (address factory) {
        factory = ICatalystV1PoolImmutables(vault).FACTORY();
        address cci = ICatalystV1PoolImmutables(vault)._chainInterface();
        // Check if the factory agree
        if (!CatalystSwapPoolFactory(factory).IsCreatedByFactory(cci, vault)) factory = address(0);
    }

    // Returns the code hash of the address.
    function get_vault_type(address vault) public view returns (bytes32) {
        return vault.codehash;
    }


    // Returns the abi_version using a whitelisted table. Returns -1 if the address is not known
    function get_pool_abi_version(address vault) public view returns (int256 abi_version) {
        abi_version = _vault_abi_version[get_vault_type(vault)];
        if (abi_version == 0) abi_version = -1;
    }


    // Returns the pool tokens supported by a pool by iterating over _tokenIndexing until it returns 0
    function get_pool_tokens(address vault) public view returns (address[] memory vaultTokens) {
        vaultTokens = new address[](ICatalystV1PoolImmutables(vault).MAX_ASSETS());
        for (uint256 it; true; ++it) {
            address token = ICatalystV1PoolImmutables(vault)._tokenIndexing(it);
            if (token == address(0)) break;
            vaultTokens[it] = token;
        }
    }


    // Returns an address which implements and exposes the pool’s mathematical methods.
    // Uses get_pool_type to find a mathematical lib. Returns address(0) if no mathematical lib is set. (whitelist)
    function get_pool_mathematical_lib(address vault) public view returns (address math_lib) {
        math_lib = ICatalystV1PoolImmutables(vault).MATHLIB();
    }


    // Returns a list of token prices. The first element is always a reference balance. (what is “1”)
    // Requires get_pool_mathematical_lib() ≠ address(0)
    function get_pool_prices(address vault) external view returns (uint256[] memory quotes) {
        quotes = new uint256[](ICatalystV1PoolImmutables(vault).MAX_ASSETS());
        address math_lib = get_pool_mathematical_lib(vault);
        if (math_lib == address(0)) return quotes;
        address[] memory tokens = get_pool_tokens(vault);
        for (uint256 it; it < tokens.length; ++it) {
            address token = tokens[it];
            quotes[it] = ICatalystMathLib(math_lib).calcAsyncPriceFrom(vault, token);
        }
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function add_whitelisted_template(address template_to_whitelist, int256 vault_abi) external onlyOwner {
        if (template_to_whitelist == address(0)) revert ZeroAddress(); 
        if (vault_abi <= 0) revert IncorrectAbi();

        // Update whitelist table
        _whitelisted_templates.push(template_to_whitelist);

        emit ModifyWhitelistedTemplate(template_to_whitelist, true);

        // Set abi
        bytes32 vault_type = get_vault_type(template_to_whitelist);
        _vault_abi_version[vault_type] = vault_abi;

        emit ModifyVaultAbi(template_to_whitelist, vault_abi);

    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function modify_vault_abi(address vault, int256 vault_abi) external onlyOwner {
        if (get_pool_abi_version(vault) == -1) revert ZeroAddress();
        if (vault_abi <= 0) revert IncorrectAbi(); 

        // Set abi
        bytes32 vault_type = get_vault_type(vault);
        _vault_abi_version[vault_type] = vault_abi;

        emit ModifyVaultAbi(vault, vault_abi);

    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function remove_whitelisted_template(address template_to_unwhitelist, uint256 template_index) external onlyOwner {
        if (template_to_unwhitelist == address(0)) revert ZeroAddress(); 
        if (_whitelisted_templates[template_index] != template_to_unwhitelist) revert InvalidIndex(template_to_unwhitelist, _whitelisted_templates[template_index]);

        delete _whitelisted_templates[template_index];

        emit ModifyWhitelistedTemplate(template_to_unwhitelist, false);
    }


    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function add_whitelisted_cii(address cci_to_whitelist) external onlyOwner {
        if (cci_to_whitelist == address(0)) revert ZeroAddress(); 

        _whitelisted_ccis.push(cci_to_whitelist);

        emit ModifyWhitelistedCCI(cci_to_whitelist, true);
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function remove_whitelisted_cci(address cci_to_whitelist, uint256 cci_index) external onlyOwner {
        if (cci_to_whitelist == address(0)) revert ZeroAddress(); 
        if (_whitelisted_ccis[cci_index] != cci_to_whitelist) revert InvalidIndex(cci_to_whitelist, _whitelisted_ccis[cci_index]);

        _whitelisted_ccis[cci_index] = address(0);

        emit ModifyWhitelistedTemplate(cci_to_whitelist, false);
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function add_pool_factory(address pool_factory) external onlyOwner {
        if (pool_factory == address(0)) revert ZeroAddress(); 

        _vault_factories.push(pool_factory);

        emit ModifyPoolFactory(pool_factory, true);
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function remove_pool_factory(address pool_factory, uint256 factory_index) external onlyOwner {
        if (pool_factory == address(0)) revert ZeroAddress(); 
        if (_vault_factories[factory_index] != pool_factory) revert InvalidIndex(pool_factory, _vault_factories[factory_index]);

        delete _vault_factories[factory_index];

        emit ModifyPoolFactory(pool_factory, false);
    }
}