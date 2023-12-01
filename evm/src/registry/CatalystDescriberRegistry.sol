//SPDX-License-Identifier: MIT

pragma solidity ^0.8.19;

import "openzeppelin-contracts/contracts/access/Ownable.sol";
import { Contains } from "./lib/Contains.sol";

/**
 * @title Catalyst: Describer Registry
 * @author Catalyst Labs
 * @notice This contract serves as an index of Catalyst Describer.
 */
contract CatalystDescriberRegistry is Contains, Ownable {
    struct AddressAndVersion {
        address addr;
        string version;
    }

    /// @notice Describes the catalyst version and the associated describer.
    event ModifyDescriber(
        address catalystDescriber,
        string version
    );

    uint256 public initBlock;

    string[] public describer_versions;
    mapping(string => address) version_to_describer;
    mapping(address => string) describer_to_version;

    address[] private _vault_describers;
    mapping(address => uint256) private _describer_version;

    constructor(address defaultOwner) {
        _transferOwnership(defaultOwner);
        initBlock = block.number;
    }

    //--- Getters ---//

    /**
    * @notice Returns all describers.
    */
    function get_vault_describers() public view returns (address[] memory catalystDescribers) {
        catalystDescribers = new address[](describer_versions.length);
        for (uint256 i = 0; i < describer_versions.length; ++i) {
            string memory version = describer_versions[i];
            catalystDescribers[i] = version_to_describer[version];
        }
    }

    /**
     * @notice Return an array of describers along with their respective version.
     * @dev Will not contain address(0), the index in the array matches the index of describer_versions.
     */
    function getVaultDescribers() external view returns (AddressAndVersion[] memory catalystDescribers) {
        catalystDescribers = new AddressAndVersion[](describer_versions.length);
        for (uint256 i = 0; i < describer_versions.length; ++i) {
            string memory version = describer_versions[i];
            catalystDescribers[i] = AddressAndVersion({
                addr: version_to_describer[version],
                version: version
            });
        }
    }

    //--- Modifiers ---//

    /**
     * @notice Sets or modifies a describer.
     * @dev describer_address cannot be 0 Instead, use the remove version.
     * @param describer_address The address of the cci which should be whitelisted.
     * @param version The version which the new cci should be set to.
     */
    function modifyDescriber(address describer_address, string calldata version) external onlyOwner {
        if (describer_address == address(0)) revert ZeroAddress();

        // Update version table
        uint256 indexOfVersion = _contains(version, describer_versions);
        if (indexOfVersion == type(uint256).max) describer_versions.push(version);

        version_to_describer[version] = describer_address;

        emit ModifyDescriber(describer_address, version);
    }


    function removeDescriber(address describer_to_remove, string calldata version) external onlyOwner {
        address read_factory = version_to_describer[version];
        if (read_factory != describer_to_remove) revert IncorrectAddress(read_factory, describer_to_remove);
        
        uint256 indexOfVersion = _contains(version, describer_versions);
        if (indexOfVersion == type(uint256).max) revert DoesNotExist();

        if (describer_versions.length > 1) {
            // swap the last element into this element's place.
            describer_versions[indexOfVersion] = describer_versions[describer_versions.length - 1];
            describer_versions.pop();
        }

        version_to_describer[version] = address(0);

        // Emit event
        emit ModifyDescriber(address(0), version);
    }

}

