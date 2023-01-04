//SPDX-License-Identifier: UNLICENSED

pragma solidity >=0.8.17 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./IbcReceiver.sol";
import "./IbcDispatcher.sol";

struct PacketMetadata {
    address target;
    address sender;
}

contract IBCEmulator is IbcDispatcher, Ownable {
    address[2] public _ports;

    event IncomingMetadata(PacketMetadata metadata);
    event IncomingPacket(IbcPacket packet);

    function registerPort() external {
        if (_ports[0] == address(0)) {
            _ports[0] = msg.sender;
        } else {
            require(_ports[1] == address(0), "NO OPEN PORTS");
            _ports[1] = msg.sender;
        }
    }

    function sendIbcPacket(
        bytes32 channelId,
        bytes calldata payload,
        uint64 timeoutBlockHeight
    ) external {
        address target;
        int128 port;
        if (msg.sender == _ports[0]) {
            target = _ports[1];
            port = 1;
        } else {
            target = _ports[0];
            port = 0;
        }
        // _packetTarget.push(PacketMetadata(target, msg.sender));

        // bytes32(abi.encode((port - 1)**2)), bytes32(abi.encode(msg.sender)),
        // bytes32(abi.encode(port)), bytes32(abi.encode(target)),
        emit IncomingMetadata(PacketMetadata(target, msg.sender));
        emit IncomingPacket(IbcPacket(
                IbcEndpoint("bytes32(abi.encode((port - 1)**2))", "0"),
                IbcEndpoint("bytes32(abi.encode(port))", "0"),
                0,
                payload,
                IbcTimeout(0, 0)
            ));
    }

    function execute(address targetContract, IbcPacket calldata packet) external {
        IbcReceiver(targetContract).onRecvPacket(packet);
    }

    function timeout(address targetContract, IbcPacket calldata packet) external {
        IbcReceiver(targetContract).onTimeoutPacket(packet);
    }

    function ack(address targetContract, IbcPacket calldata packet) external {
        IbcReceiver(targetContract).onAcknowledgementPacket(packet);
    }

}
