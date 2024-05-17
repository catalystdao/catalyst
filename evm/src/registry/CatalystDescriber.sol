//SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Ownable } from "solady/auth/Ownable.sol";
import "../interfaces/ICatalystV1VaultImmutables.sol";
import "./interfaces/ICatalystMathLibCommon.sol";
import "../interfaces/ICatalystV1Factory.sol";
import { Contains } from "./lib/Contains.sol";
/**
 * @title Catalyst: Catalyst Describer
 * @author Catalyst Labs Inc.
 * @notice This contract describes the a Catalyst implementation and serves to simplify off-chain queries.
 * As a result, the contract is not optimised for on-chain queries but rather easy of use off-chain.
 */
contract CatalystDescriber is Contains, Ownable {
    uint256 constant MAX_MEMORY_LIMIT = 64;

    struct AddressAndVersion {
        address addr;
        string version;
    }

    /** @notice Emitted when a vault template is whitelisted and unwhitelisted. */
    event ModifyTemplate(
        address templateAddress,
        string version
    );

    /** @notice Emitted when a cross-chain interface is whitelisted and unwhitelisted. */
    event ModifyWhitelistedCCI(
        address cciAddress,
        string version
    );

    /** @notice Emitted when vault is added or removed from the describer. */
    event ModifyWhitelistedFactory(
        address factoryAddress,
        string version
    );

    uint256 public initBlock;

    address public latestRouter;

    string[] public templateVersions;
    string[] public CCIVersions;
    string[] public factoryVersions;

    mapping(string => address) public versionToTemplate;
    mapping(string => address) public versionToCCI;
    mapping(string => address) public versionToFactory;


    constructor(address defaultOwner) payable {
        _initializeOwner(defaultOwner);
        initBlock = block.number;
    }

    //--- Router ---//
    function setLatestRouter(address newRouter) external onlyOwner {
        latestRouter = newRouter;
    }

    //--- Getters ---//

    /**
     * @notice Return an array of whitelisted templates along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of templateVersions.
     */
    function getWhitelistedTemplates() external view returns (AddressAndVersion[] memory whitelistedTemplates) {
        whitelistedTemplates = new AddressAndVersion[](templateVersions.length);
        for (uint256 i = 0; i < templateVersions.length; ++i) {
            string memory version = templateVersions[i];
            whitelistedTemplates[i] = AddressAndVersion({
                addr: versionToTemplate[version],
                version: version
            });
        }
    }

    /**
     * @notice Return an array of whitelisted CCIs along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of CCIVersions.
     */
    function getWhitelistedCCI() external view returns (AddressAndVersion[] memory whitelistedCCI) {
        whitelistedCCI = new AddressAndVersion[](CCIVersions.length);
        for (uint256 i = 0; i < CCIVersions.length; ++i) {
            string memory version = CCIVersions[i];
            whitelistedCCI[i] = AddressAndVersion({
                addr: versionToCCI[version],
                version: version
            });
        }
    }

    /**
     * @notice Return an array of whitelisted factories along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of factoryVersions.
     */
    function getWhitelistedFactories() external view returns (AddressAndVersion[] memory vaultFactories) {
        vaultFactories = new AddressAndVersion[](factoryVersions.length);
        for (uint256 i = 0; i < factoryVersions.length; ++i) {
            string memory version = factoryVersions[i];
            vaultFactories[i] = AddressAndVersion({
                addr: versionToFactory[version],
                version: version
            });
        }
    }

    /**
     * @notice Returns the number of whitelisted templates.
     * @dev Returns the length of templateVersions  which should contain no empty entries.
     */ 
    function getNumWhitelistedTemplates() external view returns(uint256) {
        return templateVersions.length;
    }

    /**
     * @notice Returns the number of whitelisted CCIs.
     * @dev Returns the length of CCIVersions which should contain no empty entries.
     */
    function getNumWhitelistedCcis() external view returns(uint256) {
        return CCIVersions.length;
    }

    /**
     * @notice Returns the number of whitelisted factories.
     * @dev Returns the length of factoryVersions which should contain no empty entries.
     */
    function getNumVaultFactories() external view returns (uint256) {
        return factoryVersions.length;
    }

    // -- Templates -- //

    /**
     * @notice Sets or modifies a template with a (new) address.
     * @dev templateAddress cannot be 0 Instead, use the remove version.
     * @param templateAddress Address of the template that should be whitelisted.
     * @param version Version that the new template should be set to.
     */
    function modifyWhitelistedTemplate(address templateAddress, string calldata version) external onlyOwner {
        if (templateAddress == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, templateVersions);
        if (indexOfVersion == type(uint256).max) templateVersions.push(version);

        versionToTemplate[version] = templateAddress;

        emit ModifyTemplate(templateAddress, version);
    }

    function removeWhitelistedTemplate(address templateToRemove, string calldata version) external onlyOwner {
        address read_template = versionToTemplate[version];
        if (read_template != templateToRemove) revert IncorrectAddress(read_template, templateToRemove);

        uint256 indexOfVersion = _contains(version, templateVersions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (templateVersions.length > 1) {
            // swap the last element into this element's place.
            templateVersions[indexOfVersion] = templateVersions[templateVersions.length - 1];
            templateVersions.pop();
        }

        versionToTemplate[version] = address(0);

        emit ModifyTemplate(address(0), version);
    }

    // -- Cross-Chain Interfaces -- //

    /**
     * @notice Sets or modifies a cci with a (new) address.
     * @dev cciAddress cannot be 0 Instead, use the remove version.
     * @param cciAddress Address of the cci which should be whitelisted.
     * @param version Version that the new cci should be set to.
     */
    function modifyWhitelistedCCI(address cciAddress, string calldata version) external onlyOwner {
        if (cciAddress == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, CCIVersions);
        if (indexOfVersion == type(uint256).max) CCIVersions.push(version);

        versionToCCI[version] = cciAddress;

        emit ModifyWhitelistedCCI(cciAddress, version);
    }

    function removeWhitelistedCCI(address cciToRemove, string calldata version) external onlyOwner {
        address read_cci = versionToCCI[version];
        if (read_cci != cciToRemove) revert IncorrectAddress(read_cci, cciToRemove);

        uint256 indexOfVersion = _contains(version, templateVersions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (CCIVersions.length > 1) {
            // swap the last element into this element's place.
            CCIVersions[indexOfVersion] = CCIVersions[CCIVersions.length - 1];
            CCIVersions.pop();
        }

        versionToCCI[version] = address(0);

        emit ModifyWhitelistedCCI(address(0), version);
    }

    // -- Vault Factories -- //

    /**
     * @notice Sets or modifies a factory with a (new) address.
     * @dev factoryAddress cannot be 0 Instead, use the remove version.
     * @param factoryAddress The address of the cci which should be whitelisted.
     * @param version The version which the new cci should be set to.
     */
    function modifyWhitelistedFactory(address factoryAddress, string calldata version) external onlyOwner {
        if (factoryAddress == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, factoryVersions);
        if (indexOfVersion == type(uint256).max) factoryVersions.push(version);

        versionToFactory[version] = factoryAddress;

        emit ModifyWhitelistedFactory(factoryAddress, version);
    }


    function removeWhitelistedFactory(address factoryToRemove, string calldata version) external onlyOwner {
        address read_factory = versionToFactory[version];
        if (read_factory != factoryToRemove) revert IncorrectAddress(read_factory, factoryToRemove);
        
        uint256 indexOfVersion = _contains(version, factoryVersions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (factoryVersions.length > 1) {
            // swap the last element into this element's place.
            factoryVersions[indexOfVersion] = factoryVersions[factoryVersions.length - 1];
            factoryVersions.pop();
        }

        versionToFactory[version] = address(0);

        // Emit event
        emit ModifyWhitelistedFactory(address(0), version);
    }

    //--- Helpers ---//

    /**
     * @notice Returns a vault’s factory.
     * @dev This is fetched by asking the vault which factory deployed it, then checking with the factory.
     * Returns address(0) if the `address` lies.
     * @param vault Address of the vault
     */
    function getFactoryOfVault(address vault) external view returns (address factory) {
        factory = ICatalystV1VaultImmutables(vault).FACTORY();
        address cci = ICatalystV1VaultImmutables(vault)._chainInterface();
        // Check if the factory agree
        if (!ICatalystV1Factory(factory).isCreatedByFactory(cci, vault)) factory = address(0);
    }

    /** 
     * @notice Returns the vault's tokens 
     * @dev Is found by iterating over _tokenIndexing until it returns address(0). Then resizing the array so it doesn't
     * contain address(0).
     * @param vault Vault to get tokens of.
     */
    function getVaultTokens(address vault) public view returns (address[] memory vaultTokens) {
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
     * @param vault Vault to get the mathematical lib of.
     */
    function getVaultMathematicalLib(address vault) public view returns (address math_lib) {
        math_lib = ICatalystV1VaultImmutables(vault).MATHLIB();
    }

    /** 
     * @notice Returns a list of token prices for a vault.
     * @dev To compute a token price, the ratio of 2 elements within the array has to be compared.
     * For example, to get the value of X token 1 in token 2, one should compute: quotes[1]/quptes[0]*X
     * This works cross-chain, where the output token should be numerator and input should be denominator.
     * Is implemented through getVaultTokens and get_vault_mathematical_lib.
     * @param vault Valut to get a price list of.
     * @return quotes A list of the price quotes for the tokens (getVaultTokens). Is resized.
     */
    function getVaultPrices(address vault) external view returns (uint256[] memory quotes) {
        address[] memory tokens = getVaultTokens(vault);
        quotes = new uint256[](tokens.length);
        address math_lib = getVaultMathematicalLib(vault);
        if (math_lib == address(0)) return quotes;
        for (uint256 it; it < tokens.length; ++it) {
            address token = tokens[it];
            quotes[it] = ICatalystMathLib(math_lib).calcAsyncPriceFrom(vault, token);
        }
    }
}