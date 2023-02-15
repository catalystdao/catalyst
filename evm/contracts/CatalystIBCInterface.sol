//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "./polymerase/IbcDispatcher.sol";
import "./polymerase/IbcReceiver.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./ICatalystV1Pool.sol";
import "./interfaces/ICatalystV1PoolState.sol"; // structs


/**
 * @title Catalyst: Generalised IBC Interface
 * @author Catalyst Labs
 * @notice This contract is a generalised proof of concept
 * IBC interface using an example ABI.
 * It acts as an intermediate between the swap pool and the router to
 * abstract router logic away from the swap pools. This simplifies the
 * development of the swap pools and allows Catalyst to adopt or change
 * message routers with more flexiblity.
 */
contract CatalystIBCInterface is Ownable, IbcReceiver {
    //--- ERRORS ---//
    string constant NO_CONNECTION = "No Connection";
    string constant ONLY_IBC_CALLER = "IBC enabled function";

    //--- Config ---//
    uint256 constant MAXIMUM_TIME_FOR_TX = 2 * 60 * 60; // two hours.
    address public immutable IBCDispatcher; // Set on deployment

    //-- Variables --//

    // Usage: assert self.checkConnection[chainID][from][target]
    mapping(bytes32 => mapping(bytes32 => mapping(bytes32 => bool))) public checkConnection;


    constructor(address IBCDispatcher_) {
        IBCDispatcher = IBCDispatcher_;
    }

    /// @notice Registers IBC ports for this contract.
    /// @dev The matching CatalystIBCInterface should call
    /// registerPort at the same time to establish an
    /// IBC connection between the 2 contracts.
    function registerPort() external onlyOwner {
        IbcDispatcher(IBCDispatcher).registerPort();
    }

    /**
     * @notice Configures an empty pool.
     * @param channelId The channelId to that the pool connection is approved for.
     * @param pool The pool which is allowed to call using the connection
     * @param state boolean opening or closing the channel.
     */
    function setConnection(
        bytes32 channelId,
        bytes32 pool,
        bool state
    ) external {
        // Encode the sender in bytes32.
        bytes32 msgSenderB32 = bytes32(abi.encode(msg.sender));

        checkConnection[channelId][msgSenderB32][pool] = state;
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target pool with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection mangement ensures no
     * wrong messages enter a healthy pool.
     * @param channelId The target chain identifier.
     * @param targetPool The target pool on the target chain encoded in bytes32.
     * @param targetUser recipient of the transaction on the target chain. Encoded in bytes32.
     * @param targetAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut The minimum number of returned tokens to the targetUser on the target chain.
     * @param escrowInformation The escrow information.
     * @param calldata_ Data field if a call should be made on the target chain. 
     * Should be encoded abi.encode(<address>,<data>)
     */
    function crossChainSwap(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        uint8 targetAssetIndex,
        uint256 U,
        uint256 minOut,
        TokenEscrow memory escrowInformation,
        bytes memory calldata_
    ) external {
        // Anyone can call this function. And anyone can pass the security check later,
        // but unless someone can also manage to pass the securiry check on onRecvPacket
        // they cannot drain any value.
        // As such, the very worst they can do is waste gas.

        // Catch bad messages early. If there is no connection,
        // don't send a message.
        {
            // Encode the sender in bytes32.
            bytes32 msgSenderB32 = bytes32(abi.encode(msg.sender));

            require(
                checkConnection[channelId][msgSenderB32][targetPool],
                NO_CONNECTION
            );
        }

        // To limit the number of active variables on the stack
        // calldata is encoded seperately.
        bytes memory preparedCalldata = abi.encodePacked(
            uint16(calldata_.length),
            calldata_
        );

        // To limit the number of active variables on the stack
        // escrow information is encoded seperately.
        bytes memory preparedEscrowAndCalldata = abi.encodePacked(
            escrowInformation.amount,
            abi.encode(escrowInformation.token),
            preparedCalldata
        );

        // Encode everything into:

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
            65-96 _targetUser : bytes32
            97-128 _U : uint265
            129 _assetIndex : uint8
            130-161 _minOut : uint256
            162-193 _escrowAmount : uint256
            194-225 _escrowToken : bytes32
            226-227 _customDataLength : uint16
            228-259+_customDataLength-32 _customData : bytes...
        */

        // abi.encode allways encodes to 32 bytes.
        // abi.encodePacked encodes in the smallest possible bytes.
        // 32 bytes are reserved for addresses (for chain compatibility)..
        // Ethereum addresses only use 20 bytes, so abi.encodePacked returns 20 bytes where abi.encode returns 32 bytes.
        // We want 32 just in case other chains use 32 bytes ids.
        // abi.encodePacked encodes the arguments as a concat.
        bytes memory data = abi.encodePacked(
            uint8(0),  // Swaps has context flag 0.
            abi.encode(msg.sender),
            targetPool,
            targetUser,
            U,
            uint8(targetAssetIndex),
            minOut,
            preparedEscrowAndCalldata,
            uint32(block.number % 2**32) // Makes all hashes unique. (Since the hash contains msg.sender, targetPool and blocknumber)
        );

        IbcDispatcher(IBCDispatcher).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );

    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target pool with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection mangement ensures no
     * wrong messages enter a healthy pool.
     * @param channelId The target chain identifier.
     * @param targetPool The target pool on the target chain encoded in bytes32.
     * @param targetUser recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut The minimum number of returned tokens to the targetUser on the target chain.
     * @param escrowInformation The escrow information. 
     */
    function liquiditySwap(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        uint256 U,
        uint256 minOut,
        LiquidityEscrow memory escrowInformation
    ) external {
        // Anyone can call this function. And anyone can pass the security check later,
        // but unless someone can also manage to pass the securiry check on onRecvPacket
        // they cannot drain any value.
        // As such, the very worst they can do is waste gas.

        // Catch bad messages early. If there is no connection,
        // don't send a message.
        {
            // Encode the sender in bytes32.
            bytes32 msgSenderB32 = bytes32(abi.encode(msg.sender));

            require(
                checkConnection[channelId][msgSenderB32][targetPool],
                NO_CONNECTION
            );
        }

        // abi.encode allways encodes to 32 bytes.
        // abi.encodePacked encodes in the smallest possible bytes.
        // 32 bytes are reserved for addresses (for chain compatibility)..
        // Ethereum addresses only use 20 bytes, so abi.encodePacked returns 20 bytes where abi.encode returns 32 bytes.
        // We want 32 just in case other chains use 32 bytes ids.
        // abi.encodePacked encodes the arguments as a concat.
        bytes memory data = abi.encodePacked(
            uint8(1),
            abi.encode(msg.sender),
            targetPool,
            targetUser,
            U,
            minOut,
            escrowInformation.poolTokens,
            uint8(0),
            uint32(block.number % 2**32) // Makes all hashes unique. (Since the hash contains msg.sender, targetPool and blocknumber)
        );

        IbcDispatcher(IBCDispatcher).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );
    }

    /**
     * @notice IBC Acknowledgement package handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function onAcknowledgementPacket(IbcPacket calldata packet) external {
        bytes calldata data = packet.data;
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
        */
        bytes1 _context = data[0];
        address fromPool = abi.decode(data[1:33], (address));

        // Both flag 0 and 1 messages stores U in location 97-128
        uint256 U = uint256(bytes32(data[97:129]));

        // Check if the flag 0x01 is set. If it is, it is a liquidity swap.
        if ((_context & 0x01) != 0) {
            // Delete the escrow information for liquidity swaps.
            ICatalystV1Pool(fromPool).sendLiquidityAck(
                bytes32(data[65:97]), // targetUser
                U,
                uint256(bytes32(data[161:193])), // escrowAmount
                uint32(bytes4(data[data.length-4:data.length])) // block number
            );
            return;
        } 
        // Otherwise, it is an ordinary swap.

        // Delete the escrow information for ordinary swaps.
        ICatalystV1Pool(fromPool).sendSwapAck(
            bytes32(data[65:97]), // targetUser
            U,
            uint256(bytes32(data[162:194])), // escrowAmount
            abi.decode(data[194:226], (address)), // escrowToken
            uint32(bytes4(data[data.length-4:data.length])) // block number
        );
    }

    /**
     * @notice IBC timeout package handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function onTimeoutPacket(IbcPacket calldata packet) external {
        bytes calldata data = packet.data;
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
        */
        bytes1 _context = data[0];
        address fromPool = abi.decode(data[1:33], (address)); 

        // Both flag 0 and 1 messages stores U in location 97-128
        uint256 U = uint256(bytes32(data[97:129]));

        if ((_context & 0x01) != 0) {
            // Release the liquidiy escrow.
            ICatalystV1Pool(fromPool).sendLiquidityTimeout(
                bytes32(data[65:97]), // targetUser
                U,
                uint256(bytes32(data[161:193])), // escrowAmount
                uint32(bytes4(data[data.length-4:data.length])) // block number
            );
            return;
        }

        // Release the ordinary escrow.
        ICatalystV1Pool(fromPool).sendSwapTimeout(
            bytes32(data[65:97]), // targetUser
            U,
            uint256(bytes32(data[162:194])), // escrowAmount
            abi.decode(data[194:226], (address)), // escrowToken
            uint32(bytes4(data[data.length-4:data.length])) // block number
        );
        
    }

    /**
     * @notice IBC package handler
     * @param packet The IBC packet
     */
    function onRecvPacket(IbcPacket calldata packet) external {
        bytes calldata data = packet.data;
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
        */
        bytes1 _context = data[0];
        address pool = abi.decode(data[33:65], (address));

        {
            bytes32 channelId = bytes32(packet.src.channelId);
            bytes32 fromPool = bytes32(data[1:33]);
            require(
                checkConnection[channelId][bytes32(data[33:65])][fromPool],
                NO_CONNECTION
            );
        }

        // Check if the swap is a liquidity swap.
        if ((_context & 0x01) != 0) {
            /*
                65-96 _who : bytes32
                97-128 U : uint256
                129-160 _minOut : uint256
                161-192 _escrowAmount : uint256
             */
            address who = abi.decode(data[65:97], (address));
            uint256 U = uint256(bytes32(data[97:129]));
            uint256 minOut = uint256(bytes32(data[129:161]));

            ICatalystV1Pool(pool).receiveLiquidity(
                who,
                U,
                minOut
            );
            return;
        }
        // It swap not, so it is an asset swap.
        
        /* 
            65-96 _who : bytes32
            97-128 _U : uint256
            129 _assetIndex : uint8
            130-161 _minOut : uint256
            162-193 _escrowAmount : uint256
            194-225 _escrowToken : bytes32
            226-227 _customDataLength : 
            228-259+_customDataLength-32 _customData : bytes...
        */
        // Stack limit reached:
        // uint8 assetIndex = uint8(data[129]);
        // address who = abi.decode(data[65:97], (address));
        // uint256 U = uint256(bytes32(data[97:129]));
        // uint256 minOut = uint256(bytes32(data[130:162]));

        uint16 customDataLength = uint16(bytes2(data[226:228]));

        // CCI sets customDataLength > 0 if calldata is passed.
        if (customDataLength != 0) {
            address callDataTarget = abi.decode(data[228:260], (address));
            bytes memory calldata_ = data[260:260 + customDataLength - 32];
            ICatalystV1Pool(pool).receiveSwap(
                uint8(data[129]), // assetIndex
                abi.decode(data[65:97], (address)), // who
                uint256(bytes32(data[97:129])), // U
                uint256(bytes32(data[130:162])), // minOut
                callDataTarget,
                calldata_
            );
            return;
        }
        ICatalystV1Pool(pool).receiveSwap(
            uint8(data[129]), // assetIndex
            abi.decode(data[65:97], (address)), // who
            uint256(bytes32(data[97:129])), // U
            uint256(bytes32(data[130:162])) // minOut
        );
    }
}
