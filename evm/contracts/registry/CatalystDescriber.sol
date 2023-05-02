//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import "../interfaces/ICatalystV1PoolImmutables.sol";
import "./interfaces/ICatalystMathLibCommon.sol";
import "../SwapPoolFactory.sol";
/**
 * @title Catalyst: Catalyst Describer
 * @author Catalyst Labs
 * @notice This contract describes the a Catalyst implementation and serves to simplify off-chain queries.
 * As a result, the contract is not optimised for on-chain queries but rather easy of use off-chain.
 */
contract CatalystDescriber is Ownable {
    error ZeroAddress();
    error InvalidIndex(address providedAddress, address readAddress);
    error IncorrectAbi();

    /// @notice Emitted when a vault template is whitelisted and unwhitelisted.
    event ModifyWhitelistedTemplate(
        address template,
        bool state
    );

    /// @notice Emitted when a vault's abi version is modified.
    event ModifyVaultAbi(
        address template,
        int256 abi_version
    );

    /// @notice Emitted when a cross-chain interface is whitelisted and unwhitelisted.
    event ModifyWhitelistedCCI(
        address cci,
        bool state
    );

    /// @notice Emitted when vault is added or removed from the describer.
    event ModifyVaultFactory(
        address vault_factory,
        bool state
    );

    address[] private _whitelisted_templates;

    address[] private _whitelisted_ccis;

    address[] private _vault_factories;

    mapping(bytes32 => int256) internal _vault_abi_version;


    //--- Whitelisted Templates ---//

    /**
     * @notice Returns an array of whitelisted vault templates.
     * @dev Might contain address(0).
     */ 
    function get_whitelisted_templates() external view returns (address[] memory whitelistedTemplates) {
        return _whitelisted_templates;
    }

    /**
     * @notice Returns the number of whitelisted templates.
     * @dev Returns the length of _whitelisted_templates, as such it might not be "exact".
     * Some of the whitelisted templates might have been unwhitelisted, however, they are still
     * counted in the number of whitelisteed templates.
     */ 
    function get_num_whitelisted_templates() external view returns(uint256) {
        return _whitelisted_templates.length;
    }

    /**
     * @notice Whitelist a template.
     * @param template_to_whitelist The address of the template which should be whitelisted.
     * @param vault_abi A number representing the ABI version.
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
     * @notice Unwhitelist a template.
     * @dev While only template_index is needed to unwhitelist a template, template_to_unwhitelist is used 
     * to ensure a wrong template is not mistakenly unwhitelisted.
     * @param template_to_unwhitelist The address of the template which should be unwhitelisted.
     * @param template_index The index of the whitelist to unwhitelist.
     */
    function remove_whitelisted_template(address template_to_unwhitelist, uint256 template_index) external onlyOwner {
        if (template_to_unwhitelist == address(0)) revert ZeroAddress();
        // Check that the template_index corrosponds to the expected template (template_to_unwhitelist).
        if (_whitelisted_templates[template_index] != template_to_unwhitelist) revert InvalidIndex(template_to_unwhitelist, _whitelisted_templates[template_index]);

        // Set the template to address(0);
        delete _whitelisted_templates[template_index];

        // Emit event
        emit ModifyWhitelistedTemplate(template_to_unwhitelist, false);
    }

    //--- Whitelisted Cross-Chain Interface ---//

    /**
     * @notice Returns an array of whitelisted CCIs.
     * @dev Might contain address(0).
     */ 
    function get_whitelisted_CCI() external view returns (address[] memory whitelistedCCI) {
        return _whitelisted_ccis;
    }

    /**
     * @notice Returns the number of whitelisted CCIs.
     * @dev Returns the length of _whitelisted_ccis, as such it might not be "exact".
     * Some of the whitelisted CCIs might have been unwhitelisted, however, they are still
     * counted in the number of whitelisted CCIs.
     */
    function get_num_whitelisted_ccis() external view returns(uint256) {
        return _whitelisted_ccis.length;
    }

    /**
     * @notice Whitelist a cross-chain interface.
     * @param cci_to_whitelist The address of the CCI to whitelisted.
     */
    function add_whitelisted_cii(address cci_to_whitelist) external onlyOwner {
        if (cci_to_whitelist == address(0)) revert ZeroAddress(); 

        _whitelisted_ccis.push(cci_to_whitelist);

        emit ModifyWhitelistedCCI(cci_to_whitelist, true);
    }

    /**
     * @notice Unwhitelist a cross-chain interface.
     * @dev While only template_index is needed to unwhitelist a template, template_to_unwhitelist is used 
     * to ensure a wrong template is not mistakenly unwhitelisted.
     * @param cci_to_unwhitelist The address of the CCI to whitelisted.
     * @param cci_index The index of the cci to unwhitelist.
     */
    function remove_whitelisted_cci(address cci_to_unwhitelist, uint256 cci_index) external onlyOwner {
        if (cci_to_unwhitelist == address(0)) revert ZeroAddress();
        // Check that the cci_index corrosponds to the expected cci (cci_to_unwhitelist).
        if (_whitelisted_ccis[cci_index] != cci_to_unwhitelist) revert InvalidIndex(cci_to_unwhitelist, _whitelisted_ccis[cci_index]);

        // Set the cci to address(0);
        _whitelisted_ccis[cci_index] = address(0);

        // Emit event
        emit ModifyWhitelistedTemplate(cci_to_unwhitelist, false);
    }

    //--- Vault Factories ---//

    /**
     * @notice Returns an array of vault factories
     * @dev Might contain address(0).
     */ 
    function get_vault_factories() external view returns (address[] memory vaultFactories) {
        return _vault_factories;
    }

    /**
     * @notice Returns the number of vault factories.
     * @dev Returns the length of _vault_factories, as such it might not be "exact".
     * Some of the factories might have been removed, however, they are still
     * counted in the number of factories.
     */
    function get_num_vault_factories() external view returns (uint256) {
        return _vault_factories.length;
    }

    /**
     * @notice Adds a vault factory to the list of vault factories.
     * @param vault_factory Adds a vault factory.
     */
    function add_vault_factory(address vault_factory) external onlyOwner {
        if (vault_factory == address(0)) revert ZeroAddress(); 

        _vault_factories.push(vault_factory);

        emit ModifyVaultFactory(vault_factory, true);
    }

    /**
     * @notice Remove a vault factory from the list of vault factories.
     * @dev While only factory_index is needed to remove a factory, vault_factory is used 
     * to ensure a wrong factory is not mistakenly removed.
     * @param vault_factory The address of the factory to remove.
     * @param factory_index The index of the factory to remove.
     */
    function remove_vault_factory(address vault_factory, uint256 factory_index) external onlyOwner {
        if (vault_factory == address(0)) revert ZeroAddress();
        // Check that the factory_index corrosponds to the expected cci (vault_factory).
        if (_vault_factories[factory_index] != vault_factory) revert InvalidIndex(vault_factory, _vault_factories[factory_index]);

        // Set the factory to address(0);
        delete _vault_factories[factory_index];

        // Emit event
        emit ModifyVaultFactory(vault_factory, false);
    }

    /**
     * @notice Returns a vault’s factory.
     * @dev This is fetched by asking the vault which factory deployed it, then checking with the factory.
     * Returns address(0) if the `address` lies.
     * @param vault The address of the vault
     */
    function get_factory_of_vault(address vault) external view returns (address factory) {
        factory = ICatalystV1PoolImmutables(vault).FACTORY();
        address cci = ICatalystV1PoolImmutables(vault)._chainInterface();
        // Check if the factory agree
        if (!CatalystSwapPoolFactory(factory).IsCreatedByFactory(cci, vault)) factory = address(0);
    }


    //--- Vault ABI ---//

    /** 
     * @notice Returns the code hash of the address.
     * @dev Is a wrapper around (address).codehash;
     * @param vault Address of the vault to get codehash of
     */
    function get_vault_type(address vault) public view returns (bytes32) {
        return vault.codehash;
    }


    /** 
     * @notice Returns the abi_version for a vault
     * @dev Uses a whitelist table and returns -1 if the address is not known 
     * @param vault Address of the vault to get abi version of.
     */
    function get_vault_abi_version(address vault) public view returns (int256 abi_version) {
        abi_version = _vault_abi_version[get_vault_type(vault)];
        if (abi_version == 0) abi_version = -1;
    }

    /**
     * @notice Updates a vault's abi version
     * @param vault Address of the vault to update the abi version.
     * @param vault_abi The new abi version.
     */
    function modify_vault_abi(address vault, int256 vault_abi) external onlyOwner {
        if (get_vault_abi_version(vault) == -1) revert ZeroAddress();
        if (vault_abi <= 0) revert IncorrectAbi(); 

        // Set abi
        bytes32 vault_type = get_vault_type(vault);
        _vault_abi_version[vault_type] = vault_abi;

        emit ModifyVaultAbi(vault, vault_abi);

    }


    //--- Helper Functions for Vaults ---//

    /** 
     * @notice Returns the vault's tokens 
     * @dev Is found by iterating over _tokenIndexing until it returns address(0). Then resizing the array so it doesn't
     * contain address(0).
     * @param vault The vault to get tokens of.
     */
    function get_vault_tokens(address vault) public view returns (address[] memory vaultTokens) {
        address[] memory tempVaultTokens = new address[](ICatalystV1PoolImmutables(vault).MAX_ASSETS());
        uint256 it;
        for (it = 0; true; ++it) {
            address token = ICatalystV1PoolImmutables(vault)._tokenIndexing(it);
            if (token == address(0)) break;
            tempVaultTokens[it] = token;
        }
        // Resize array
        vaultTokens = new address[](it);
        for (uint256 i; i < it; ++i) {
            vaultTokens[i] = tempVaultTokens[i];
        }
    }

    /** 
     * @notice Returns a mathematical library which implements helpers for the contract.
     * @dev Queries the vault for the mathematical library.
     * @param vault The vault to get the mathematical lib of.
     */
    function get_vault_mathematical_lib(address vault) public view returns (address math_lib) {
        math_lib = ICatalystV1PoolImmutables(vault).MATHLIB();
    }


    /** 
     * @notice Returns a list of token prices for a vault.
     * @dev To compute a token price, the ratio of 2 elements within the array has to be compared.
     * For example, to get the value of X token 1 in token 2, one should compute: quotes[1]/quptes[0]*X
     * This works cross-chain, where the output token should be numerator and input should be denominator.
     * Is implemented through get_vault_tokens and get_vault_mathematical_lib.
     * @param vault The valut to get a price list of.
     * @return quotes A list of the price quotes for the tokens (get_vault_tokens). Is resized.
     */
    function get_vault_prices(address vault) external view returns (uint256[] memory quotes) {
        address[] memory tokens = get_vault_tokens(vault);
        quotes = new uint256[](tokens.length);
        address math_lib = get_vault_mathematical_lib(vault);
        if (math_lib == address(0)) return quotes;
        for (uint256 it; it < tokens.length; ++it) {
            address token = tokens[it];
            quotes[it] = ICatalystMathLib(math_lib).calcAsyncPriceFrom(vault, token);
        }
    }
}