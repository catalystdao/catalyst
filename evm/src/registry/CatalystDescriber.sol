//SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Ownable } from "solady/auth/Ownable.sol";
import "../interfaces/ICatalystV1VaultImmutables.sol";
import "./interfaces/ICatalystMathLibCommon.sol";
import "../interfaces/ICatalystV1Factory.sol";
import { Contains } from "./lib/Contains.sol";
/**
 * @title Catalyst: Catalyst Describer
 * @author Catalyst Labs
 * @notice This contract describes the a Catalyst implementation and serves to simplify off-chain queries.
 * As a result, the contract is not optimised for on-chain queries but rather easy of use off-chain.
 */
contract CatalystDescriber is Contains, Ownable {
    uint256 constant MAX_MEMORY_LIMIT = 64;

    struct AddressAndVersion {
        address addr;
        string version;
    }

    /// @notice Emitted when a vault template is whitelisted and unwhitelisted.
    event ModifyTemplate(
        address template_address,
        string version
    );

    /// @notice Emitted when a cross-chain interface is whitelisted and unwhitelisted.
    event ModifyWhitelistedCCI(
        address cci_address,
        string version
    );

    /// @notice Emitted when vault is added or removed from the describer.
    event ModifyWhitelistedFactory(
        address factory_address,
        string version
    );

    uint256 public initBlock;

    address public latestRouter;

    string[] public template_versions;
    string[] public cci_versions;
    string[] public factory_versions;

    mapping(string => address) public version_to_template;
    mapping(string => address) public version_to_cci;
    mapping(string => address) public version_to_factory;


    constructor(address defaultOwner) payable {
        _initializeOwner(defaultOwner);
        initBlock = block.number;
    }

    //--- Router ---//
    function set_latest_router(address newRouter) external onlyOwner {
        latestRouter = newRouter;
    }

    //--- Getters ---//

    /**
     * @notice Return an array of whitelisted templates. The index in the array matches the index of template_versions.
     * @dev Will not contain address(0).
     */
    function get_whitelisted_templates() external view returns (address[] memory whitelistedTemplates) {
        whitelistedTemplates = new address[](template_versions.length);
        for (uint256 i = 0; i < template_versions.length; ++i) {
            string memory version = template_versions[i];
            whitelistedTemplates[i] =  version_to_template[version];
        }
    }

    /**
     * @notice Return an array of whitelisted templates along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of template_versions.
     */
    function getWhitelistedTemplates() external view returns (AddressAndVersion[] memory whitelistedTemplates) {
        whitelistedTemplates = new AddressAndVersion[](template_versions.length);
        for (uint256 i = 0; i < template_versions.length; ++i) {
            string memory version = template_versions[i];
            whitelistedTemplates[i] = AddressAndVersion({
                addr: version_to_template[version],
                version: version
            });
        }
    }

    /**
     * @notice Return an array of whitelisted CCIs along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of cci_versions.
     */
    function get_whitelisted_CCI() external view returns (AddressAndVersion[] memory whitelistedCCI) {
        whitelistedCCI = new AddressAndVersion[](cci_versions.length);
        for (uint256 i = 0; i < cci_versions.length; ++i) {
            string memory version = cci_versions[i];
            whitelistedCCI[i] = AddressAndVersion({
                addr: version_to_cci[version],
                version: version
            });
        }
    }

    /**
     * @notice Return an array of whitelisted CCIs along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of cci_versions.
     */
    function getWhitelistedCCI() external view returns (AddressAndVersion[] memory whitelistedCCI) {
        whitelistedCCI = new AddressAndVersion[](cci_versions.length);
        for (uint256 i = 0; i < cci_versions.length; ++i) {
            string memory version = cci_versions[i];
            whitelistedCCI[i] = AddressAndVersion({
                addr: version_to_cci[version],
                version: version
            });
        }
    }

    /**
     * @notice Returns an array of whitelisted factories. The index in the array matches the index of factory_versions.
     * @dev Will not contain address(0).
     */ 
    function get_vault_factories() external view returns (address[] memory vaultFactories) {
        vaultFactories = new address[](factory_versions.length);
        for (uint256 i = 0; i < factory_versions.length; ++i) {
            string memory version = factory_versions[i];
            vaultFactories[i] = version_to_factory[version];
        }
    }

    /**
     * @notice Return an array of whitelisted factories along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of factory_versions.
     */
    function getWhitelistedFactories() external view returns (AddressAndVersion[] memory vaultFactories) {
        vaultFactories = new AddressAndVersion[](factory_versions.length);
        for (uint256 i = 0; i < factory_versions.length; ++i) {
            string memory version = factory_versions[i];
            vaultFactories[i] = AddressAndVersion({
                addr: version_to_factory[version],
                version: version
            });
        }
    }

    /**
     * @notice Returns the number of whitelisted templates.
     * @dev Returns the length of template_versions  which should contain no empty entries.
     */ 
    function get_num_whitelisted_templates() external view returns(uint256) {
        return template_versions.length;
    }

    /**
     * @notice Returns the number of whitelisted templates.
     * @dev Returns the length of template_versions  which should contain no empty entries.
     */ 
    function getNumWhitelistedTemplates() external view returns(uint256) {
        return template_versions.length;
    }

    /**
     * @notice Returns the number of whitelisted CCIs.
     * @dev Returns the length of cci_versions which should contain no empty entries.
     */
    function get_num_whitelisted_ccis() external view returns(uint256) {
        return cci_versions.length;
    }

    /**
     * @notice Returns the number of whitelisted CCIs.
     * @dev Returns the length of cci_versions which should contain no empty entries.
     */
    function getNumWhitelistedCcis() external view returns(uint256) {
        return cci_versions.length;
    }


    /**
     * @notice Returns the number of whitelisted factories.
     * @dev Returns the length of factory_versions which should contain no empty entries.
     */
    function get_num_vault_factories() external view returns (uint256) {
        return factory_versions.length;
    }

    /**
     * @notice Returns the number of whitelisted factories.
     * @dev Returns the length of factory_versions which should contain no empty entries.
     */
    function getNumVaultFactories() external view returns (uint256) {
        return factory_versions.length;
    }

    //--- Modifiers ---//

    // -- Templates -- //

    /**
     * @notice Sets or modifies a template with a (new) address.
     * @dev template_address cannot be 0 Instead, use the remove version.
     * @param template_address The address of the template which should be whitelisted.
     * @param version The version which the new template should be set to.
     */
    function modifyWhitelistedTemplate(address template_address, string calldata version) external onlyOwner {
        if (template_address == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, template_versions);
        if (indexOfVersion == type(uint256).max) template_versions.push(version);

        version_to_template[version] = template_address;

        emit ModifyTemplate(template_address, version);
    }

    function removeWhitelistedTemplate(address template_to_remove, string calldata version) external onlyOwner {
        address read_template = version_to_template[version];
        if (read_template != template_to_remove) revert IncorrectAddress(read_template, template_to_remove);

        uint256 indexOfVersion = _contains(version, template_versions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (template_versions.length > 1) {
            // swap the last element into this element's place.
            template_versions[indexOfVersion] = template_versions[template_versions.length - 1];
            template_versions.pop();
        }

        version_to_template[version] = address(0);

        emit ModifyTemplate(address(0), version);
    }

    // -- Cross-Chain Interfaces -- //

    /**
     * @notice Sets or modifies a cci with a (new) address.
     * @dev cci_address cannot be 0 Instead, use the remove version.
     * @param cci_address The address of the cci which should be whitelisted.
     * @param version The version which the new cci should be set to.
     */
    function modifyWhitelistedCCI(address cci_address, string calldata version) external onlyOwner {
        if (cci_address == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, cci_versions);
        if (indexOfVersion == type(uint256).max) cci_versions.push(version);

        version_to_cci[version] = cci_address;

        emit ModifyWhitelistedCCI(cci_address, version);
    }

    function removeWhitelistedCCI(address cci_to_remove, string calldata version) external onlyOwner {
        address read_cci = version_to_cci[version];
        if (read_cci != cci_to_remove) revert IncorrectAddress(read_cci, cci_to_remove);

        uint256 indexOfVersion = _contains(version, template_versions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (cci_versions.length > 1) {
            // swap the last element into this element's place.
            cci_versions[indexOfVersion] = cci_versions[cci_versions.length - 1];
            cci_versions.pop();
        }

        version_to_cci[version] = address(0);

        emit ModifyWhitelistedCCI(address(0), version);
    }

    // -- Vault Factories -- //

    /**
     * @notice Sets or modifies a factory with a (new) address.
     * @dev factory_address cannot be 0 Instead, use the remove version.
     * @param factory_address The address of the cci which should be whitelisted.
     * @param version The version which the new cci should be set to.
     */
    function modifyWhitelistedFactory(address factory_address, string calldata version) external onlyOwner {
        if (factory_address == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, factory_versions);
        if (indexOfVersion == type(uint256).max) factory_versions.push(version);

        version_to_factory[version] = factory_address;

        emit ModifyWhitelistedFactory(factory_address, version);
    }


    function removeWhitelistedFactory(address factory_to_remove, string calldata version) external onlyOwner {
        address read_factory = version_to_factory[version];
        if (read_factory != factory_to_remove) revert IncorrectAddress(read_factory, factory_to_remove);
        
        uint256 indexOfVersion = _contains(version, factory_versions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (factory_versions.length > 1) {
            // swap the last element into this element's place.
            factory_versions[indexOfVersion] = factory_versions[factory_versions.length - 1];
            factory_versions.pop();
        }

        version_to_factory[version] = address(0);

        // Emit event
        emit ModifyWhitelistedFactory(address(0), version);
    }

    //--- Helpers ---//

    /**
     * @notice Returns a vault’s factory.
     * @dev This is fetched by asking the vault which factory deployed it, then checking with the factory.
     * Returns address(0) if the `address` lies.
     * @param vault The address of the vault
     */
    function get_factory_of_vault(address vault) external view returns (address factory) {
        factory = ICatalystV1VaultImmutables(vault).FACTORY();
        address cci = ICatalystV1VaultImmutables(vault)._chainInterface();
        // Check if the factory agree
        if (!ICatalystV1Factory(factory).isCreatedByFactory(cci, vault)) factory = address(0);
    }

    /** 
     * @notice Returns the vault's tokens 
     * @dev Is found by iterating over _tokenIndexing until it returns address(0). Then resizing the array so it doesn't
     * contain address(0).
     * @param vault The vault to get tokens of.
     */
    function get_vault_tokens(address vault) public view returns (address[] memory vaultTokens) {
        address[] memory tempVaultTokens = new address[](MAX_MEMORY_LIMIT);
        uint256 it;
        for (it = 0; true; ++it) {
            address token = ICatalystV1VaultImmutables(vault)._tokenIndexing(it);
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
        math_lib = ICatalystV1VaultImmutables(vault).MATHLIB();
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