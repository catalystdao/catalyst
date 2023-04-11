//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";

contract CatalystDescriberRegistry is Ownable {
    error WrongCatalystVersion(uint256 proposed, uint256 actual);
    error ZeroDescriber();

    event CatalystDescriber(
        uint256 catalystVersion,
        address catalystDescriber
    );

    mapping(uint256 => address) private _pool_describer;
    mapping(address => uint256) private _describer_version;
    uint256 public catalystVersions;


    /** 
    * Given a Catalyst version, returns the current pool describer.
    * @dev Returns address(0) if no describer exists.
    */
    function get_pool_describer(uint256 catalystVersion) public view returns(address) {
        return _pool_describer[catalystVersion];
    }

    /**
     * Given a pool describer, returns the catalyst version. 
     * @dev Returns 0 if address is not a CatalystDescriber.
     */
    function get_describer_version(address catalystDescriber) external view returns (uint256) {
        return _describer_version[catalystDescriber];
    }

    /**
    * @notice Returns all CatalystDescribers.
    */
    function get_pool_describers() public view returns (address[] memory catalystDescribers) {
        for (uint256 it; it <= catalystVersions; ++it) {
            address catalystDescriber = get_pool_describer(it);
            catalystDescribers[it] = catalystDescriber;
        }
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function set_describer(uint256 catalystVersion, address catalystDescriber) external onlyOwner {
        uint256 nextCatalystVersion = catalystVersions + 1;
        if (nextCatalystVersion != catalystVersion) revert  WrongCatalystVersion(catalystVersion, nextCatalystVersion); 
        if (catalystDescriber == address(0)) revert  ZeroDescriber(); 

        _describer_version[catalystDescriber] = catalystVersion;
        _pool_describer[catalystVersion] = catalystDescriber;
        catalystVersions += 1;

        emit CatalystDescriber(catalystVersion, catalystDescriber);
    }

}

