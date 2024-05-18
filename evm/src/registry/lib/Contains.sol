//SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title Catalyst: Catalyst Describer
 * @author Catalyst Labs Inc.
 * @notice This contract describes the a Catalyst implementation and serves to simplify off-chain queries.
 * As a result, the contract is not optimised for on-chain queries but rather easy of use off-chain.
 */
abstract contract Contains {
    error ZeroAddress();
    error IncorrectAddress(address expected, address provided);
    error DoesNotExist();

    /**
     * @notice Finds target inside arr. Returns type(uint256).max if the target isn't found.
     */
    function _contains(string memory target, string[] memory arr) internal pure returns(uint256 index) {
        uint256 num_versions = arr.length;
        bytes32 hash_of_target = keccak256(abi.encodePacked(target));
        for (index = 0; index < num_versions;) {
            if (hash_of_target == keccak256(abi.encodePacked(arr[index]))) {
                return index;
            }

            unchecked {
                ++index;
            }
        }
        return index = type(uint256).max;
    }
}