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

    address[] private _pool_describers;
    mapping(address => uint256) private _describer_version;
    uint256 public catalystVersions;


    /** 
    * Given a Catalyst version, returns the current pool describer.
    * @dev Returns address(0) if no describer exists.
    */
    function get_pool_describer(uint256 catalystVersion) public view returns(address) {
        if (_pool_describers.length <= catalystVersion) return address(0);
        return _pool_describers[catalystVersion];
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
        return _pool_describers;
    }

    /**
     * @notice Defines a new Catalyst Describer and incremenets the Catalyst version
     */
    function add_describer(address catalystDescriber) external onlyOwner {
        if (catalystDescriber == address(0)) revert  ZeroDescriber(); 

        _pool_describers.push(catalystDescriber);
        _describer_version[catalystDescriber] = _pool_describers.length;

        emit CatalystDescriber(_pool_describers.length, catalystDescriber);
    }

}

