//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.9;

import "@openzeppelin/contracts/access/Ownable.sol";

interface ITarget {
    function receives(bytes memory _header, bytes memory _data) external;
}

/// @title Polymerase Interface
/// @author Polymer Labs
/// @notice 
///     1. To provide a way for contracts to easily interact with
///     Polymerase by providing a reference for how to emit the events.
///     2. To provide a way to securely accept incoming calls to receives.
///     The contract can be provided be entrusted with the sole permission
///     for calling receives(...).
contract PolymeraseInterface is Ownable {
    
    /// @notice Provides Polymerase with the data to execute
    /// @dev {{Can be implemented anywhere}}
    /// @param _chain indexed uint64; The destination chain using the Polymerase lookup table.
    /// @param _target indexed bytes32; The target contract on the destination chain. If possible, the byte representation of the address otherwise the {{HASH}} of the address.
    /// @param _data bytes; The data to deliver.
    event CrossChainTX(
        uint64 indexed _chain,
        bytes32 indexed _target,
        bytes _data
    );


    /// @notice Emmited when Polymerase delivers data
    /// @dev Header length is 40 bytes.
    /// @param _target indexed address; The address where data has been delivered
    /// @param _header bytes;
    ///        allocatedBytes, Length, Type: representation
    ///        0-7, 8, uint64: ChainID
    ///        8-39, 32, bytes32: Calle  # If an address can be stored in 
    /// @param _data bytes; The data delivered.
    event Executed(
        address indexed _target,
        bytes _header,
        bytes _data
    );

    /// @notice Wraps emitting the correct Polymerase event.
    /// @dev 
    /// @param _chain uint64; The destination chain using the Polymerase lookup table.
    /// @param _target bytes32; The target contract on the destination chain. If possible, the byte representation of the address otherwise the {{HASH}} of the address.
    /// @param _data bytes; The data to deliver.
    function call_multichain(
        uint64 _chain,
        bytes32 _target,
        bytes memory _data
    ) external {
        emit CrossChainTX(_chain, _target, _data);
    }


    /// @notice Used by the Polymerase network for delivering onchain calls.
    /// @dev Only callable by the Polymerase network.
    /// @param _target address; The address to call receives(...) on.
    /// @param _header bytes;
    ///        allocatedBytes, Length, Type: representation
    ///        0-7, 8, uint64: ChainID
    ///        8-39, 32, bytes32: Calle  # If an address can be stored in 32 bytes, calle address otherwise HASH of calle address
    /// @param _data bytes; Bytes to deliver.
    function execute(address _target, bytes memory _header, bytes memory _data) external onlyOwner {
        ITarget(_target).receives(_header, _data);
        emit Executed(_target, _header, _data);
    }
}
