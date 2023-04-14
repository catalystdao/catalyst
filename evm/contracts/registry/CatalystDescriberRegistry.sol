//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title Catalyst: Describer Registry
 * @author Catalyst Labs
 * @notice This contract serves as an index of Catalyst Describer.
 */
contract CatalystDescriberRegistry is Ownable {
    error WrongCatalystVersion(uint256 proposed, uint256 actual);
    error ZeroDescriber();

    /// @notice Describes the catalyst version and the associated describer.
    event CatalystDescriber(
        uint256 catalystVersion,
        address catalystDescriber
    );

    address[] private _vault_describers;
    mapping(address => uint256) private _describer_version;


    /** 
    * @notice Given a Catalyst version, returns the current vault describer.
    * @dev Returns address(0) if no describer exists.
    * @param catalystVersion The Catalyst version. Is 1 indexed.
    */
    function get_vault_describer(uint256 catalystVersion) public view returns(address) {
        if (_vault_describers.length <= catalystVersion) return address(0);
        return _vault_describers[catalystVersion];
    }

    /** 
    * @notice Returns the current catalyst version.
    * @dev Returns the length of _vault_describers. 
    * To get the latest describer: get_vault_describer(catalyst_version())
    */
    function catalyst_version() public view returns(uint256) {
        return _vault_describers.length;
    }

    /**
     * @notice Given a vault describer, returns the catalyst version. 
     * @dev Returns 0 if address is not a CatalystDescriber.
     * It might be that get_vault_describer(get_describer_version(catalystDescriber)) != catalystDescriber,
     * since when a describer is updated it doesn't delete the index.
     * @param catalystDescriber The address of the catalyst describer
     */
    function get_describer_version(address catalystDescriber) external view returns (uint256) {
        return _describer_version[catalystDescriber];
    }

    /**
    * @notice Returns all CatalystDescribers.
    */
    function get_vault_describers() public view returns (address[] memory catalystDescribers) {
        return _vault_describers;
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     * @param catalystDescriber The address of the catalyst describer to use for the new version
     */
    function add_describer(address catalystDescriber) external onlyOwner {
        if (catalystDescriber == address(0)) revert  ZeroDescriber(); 

        _vault_describers.push(catalystDescriber);
        _describer_version[catalystDescriber] = _vault_describers.length;

        emit CatalystDescriber(_vault_describers.length, catalystDescriber);
    }

    /**
     * @notice Updates a Catalyst version with a new describer.
     * @dev Doesn't reset _describer_version for the old describer. 
     * @param catalystDescriber The address of the new describer
     * @param catalystVersion The version which the new describer will overwrite.
     */
    function modify_describer(address catalystDescriber, uint256 catalystVersion) external onlyOwner {
        if (catalystDescriber == address(0)) revert ZeroDescriber(); 

        _vault_describers[catalystVersion] = catalystDescriber;
        _describer_version[catalystDescriber] = catalystVersion;

        emit CatalystDescriber(catalystVersion, catalystDescriber);
    }

}

