//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";

import "./IbcDispatcher.sol";

/// @title Dispatcher
/// @author Polymer Labs
/// @notice
///     Contract callers call this contract to send IBC-like msg,
///     which can be relayed to a rollup module on the Polymerase chain
contract Dispatcher is Ownable, IbcDispatcher {
    event IbcPacket(
        address indexed sender,
        bytes32 indexed channelId,
        bytes payload,
        uint64 timeoutBlockHeight
    );

    event PortRegistration(address indexed sender);

    function registerPort() external {
        emit PortRegistration(msg.sender);
    }

    /// Sends an IBC packet with given data over the existing channel.
    /// Data should be encoded in a format defined by the channel version,
    /// and the module on the other side should know how to parse this.
    function sendIbcPacket(
        bytes32 channelId,
        bytes calldata payload,
        uint64 timeoutBlockHeight
    ) external {
        emit IbcPacket(msg.sender, channelId, payload, timeoutBlockHeight);
    }
}
