//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./IbcReceiver.sol";
import "./IbcDispatcher.sol";

contract IBCEmulator is IbcDispatcher, Ownable {
    bytes32 public immutable LOCALCHANNELID;

    event Packet(IbcPacket packet);

    event Acknowledgement(bytes acknowledgement);

    constructor(bytes32 localChannelId) {
        LOCALCHANNELID = localChannelId;
    }

    function registerPort() external {}

    function sendIbcPacket(
        bytes32 channelId,
        bytes calldata payload,
        uint64 timeoutBlockHeight
    ) external {
        bytes32 fromChannel = (LOCALCHANNELID << 128) ^ channelId;
        bytes32 toChannel = channelId;
        emit Packet(
            IbcPacket(
                IbcEndpoint(0, fromChannel),
                IbcEndpoint(0, toChannel),
                0,
                payload,
                IbcTimeout(timeoutBlockHeight, 0)
            )
        );
    }

    function execute(address targetContract, IbcPacket calldata packet)
        external onlyOwner
    {
        bytes memory acknowledgement = IbcReceiver(targetContract).onRecvPacket(packet);

        emit Acknowledgement(acknowledgement);
    }

    function timeout(address targetContract, IbcPacket calldata packet)
        external onlyOwner
    {
        IbcReceiver(targetContract).onTimeoutPacket(packet);
    }

    function ack(address targetContract, bytes calldata acknowledgement, IbcPacket calldata packet) external {
        IbcReceiver(targetContract).onAcknowledgementPacket(acknowledgement, packet);
    }
}