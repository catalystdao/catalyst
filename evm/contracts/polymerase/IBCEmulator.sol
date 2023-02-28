//SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./IbcReceiver.sol";
import "./IbcDispatcher.sol";

contract IBCEmulator is IbcDispatcher, Ownable {
    event Packet(IbcPacket packet);

    function registerPort() external {}

    function sendIbcPacket(
        bytes32 channelId,
        bytes calldata payload,
        uint64 timeoutBlockHeight
    ) external {
        emit Packet(
            IbcPacket(
                IbcEndpoint(0, channelId),
                IbcEndpoint(0, channelId),
                0,
                payload,
                IbcTimeout(timeoutBlockHeight, 0)
            )
        );
    }

    function execute(address targetContract, IbcPacket calldata packet)
        external onlyOwner
    {
        IbcReceiver(targetContract).onRecvPacket(packet);
    }

    function timeout(address targetContract, IbcPacket calldata packet)
        external onlyOwner
    {
        IbcReceiver(targetContract).onTimeoutPacket(packet);
    }

    function ack(address targetContract, IbcPacket calldata packet) external onlyOwner {
        IbcReceiver(targetContract).onAcknowledgementPacket(packet);
    }
}
