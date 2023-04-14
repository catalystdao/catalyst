//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

struct IbcEndpoint {
    bytes32 portId;
    bytes32 channelId;
}

/// In IBC each package must set at least one type of timeout:
/// the timestamp or the block height.
struct IbcTimeout {
    uint64 block;
    uint64 timestamp;
}

struct IbcPacket {
    /// identifies the channel and port on the sending chain.
    IbcEndpoint src;
    /// identifies the channel and port on the receiving chain.
    IbcEndpoint dest;
    /// The sequence number of the packet on the given channel
    uint64 sequence;
    bytes data;
    /// when packet times out, measured on remote chain
    IbcTimeout timeout; // ! move this up over Sequence to take advantage of packing.
}

interface IbcReceiver {
    function onRecvPacket(IbcPacket calldata packet) external;

    function onAcknowledgementPacket(bytes calldata acknowledgement, IbcPacket calldata packet) external;

    function onTimeoutPacket(IbcPacket calldata packet) external;
}
