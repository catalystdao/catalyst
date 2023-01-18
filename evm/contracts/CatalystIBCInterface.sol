//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "./polymerase/IbcDispatcher.sol";
import "./polymerase/IbcReceiver.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./ICatalystV1Pool.sol";
import "./interfaces/ICatalystV1PoolState.sol"; // structs

interface ISwapPoolFactory {
    function IsCreatedByFactory(address arg0, address arg1)
        external
        view
        returns (bool);
}

/// @title CatalystIBCInterface
/// @author Alexander @ Polymer Labs
/// @notice Interface for PolySwap for sending IBC messages using Polymerase
/**
 * Context Flag
 *      2^0: Asset (0) or Liquidity swap (1)
 *        1: Non-approximate (0) or approximate (1)
 *        2: Unused
 *        3: Unused
 *        .
 *        .
 *        7: Unused
 */
contract CatalystIBCInterface is Ownable, IbcReceiver {
    //--- ERRORS ---//
    string constant NO_CONNECTION = "Pools are not connected";
    string constant SWAP_RETURN_INSUFFICIENT = "Insufficient swap return";
    string constant BALANCE_SECURITY_LIMIT =
        "Pool sanity limit (Balance too large)";

    uint256 constant MAXIMUM_TIME_FOR_TX = 60 * 60; // One hour.

    address public immutable IBCDispatcher;
    mapping(address => bool) public validArriver;

    // Usage: assert self.checkConnection[chainID][from][target]
    mapping(bytes32 => mapping(bytes32 => mapping(bytes32 => bool)))
        public checkConnection;

    mapping(uint256 => bytes32) public _channelForChain;

    address public immutable _calldataRouter;

    constructor(
        address IBCDispatcher_
    ) {
        IBCDispatcher = IBCDispatcher_;
        validArriver[IBCDispatcher] = true;
        _calldataRouter = address(0);
    }

    function registerPort() external onlyOwner {
        IbcDispatcher(IBCDispatcher).registerPort();
    }

    function chain_id() external view returns (uint256) {
        return block.chainid;
    }

    function CreateConnection(
        bytes32 channelId,
        bytes32 pool,
        bool state
    ) external returns (bool) {
        bytes32 msgSenderB32 = bytes32(abi.encodePacked(msg.sender));

        checkConnection[channelId][msgSenderB32][pool] = state;
        checkConnection[channelId][pool][msgSenderB32] = state;

        return true;
    }

    function crossChainSwap(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        uint8 targetAssetIndex,
        uint256 U,
        uint256 minOut,
        TokenEscrow memory escrowInformation,
        bytes memory calldata_
    ) external returns (bytes32) {
        /**
         * Context Flag
         *      2^0: Asset (0) or Liquidity swap (1)
         *        1: Non-approximate (0) or approximate (1)
         *        2: Unused
         *        3: Unused
         *        .
         *        .
         *        7: Unused
         */

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
            65-96 _targetUser : bytes32
            97-128 _U : uint265
            129 _assetIndex : uint8
            130-161 _minOut : uint256
            162-193 _escrowAmount : uint256
            194-214 _escrowToken
            215-216 _customDataLength : uint16  # If custom data is passed.
            217-248+_customDataLength-1 _customData : bytes...
        */

        bytes memory preparedCalldata = abi.encodePacked(
            uint16(calldata_.length),
            calldata_
        );

        bytes memory preparedEscrowAndCalldata = abi.encodePacked(
            escrowInformation.amount,
            abi.encode(escrowInformation.token),
            preparedCalldata
        );

        // abi.encodePacked encodes the arguments as a concat.
        bytes memory data = abi.encodePacked(
            uint8(0),
            abi.encode(msg.sender),
            targetPool,
            targetUser,
            U,
            uint8(targetAssetIndex),
            minOut,
            preparedEscrowAndCalldata,
            uint32(block.number % 2**32) // Makes all hashes unique. (Since the hash contains msg.sender, targetPool and blocknumber)
        );
        // abi.encode allways encodes to 32 bytes.
        // abi.encodePacked encodes in the smallest possible bytes.
        // 32 bytes are reserved for addresses (for chain compatibility)..
        // Ethereum addresses only use 20 bytes, so abi.encodePacked returns 20 bytes where abi.encode returns 32 bytes.

        IbcDispatcher(IBCDispatcher).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );

        return keccak256(data);
    }

    function liquiditySwap(
        bytes32 channelId,
        bytes32 targetPool,
        bytes32 targetUser,
        uint256 U,
        uint256 minOut,
        LiquidityEscrow memory escrowInformation
    ) external returns (bytes32) {
        /*
         * Context Flag
         *      2^0: Asset (0) or Liquidity swap (1)
         *        1: Non-approximate (0) or approximate (1)
         *        2: Unused
         *        3: Unused
         *        .
         *        .
         *        7: Unused
         */

        /*  // TODO
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
            65-96 _targetUser : bytes32
            97-128 _U : uint265
            130-161 _minOut : uint256
            162-193 _escrowAmount : uint256
        */

        // require(
        //     checkConnection[channelId][bytes32(abi.encodePacked(msg.sender))][
        //         _pool
        //     ],
        //     NO_CONNECTION
        // );

        // abi.encodePacked encodes the arguments as a concat. This should be the same as:
        // _data: Bytes[130] = concat(0x00,  convert(msg.sender, bytes32), _pool, _who, assetIndex, C) in vyper
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
        // abi.encode always encodes to 32 bytes.
        // abi.encodePacked encodes in the smallest possible bytes.
        // 32 bytes are reserved for addresses (for chain compatibility).
        // Ethereum addresses only use 20 bytes, so abi.encodePacked returns 20 bytes where abi.encode returns 32 bytes.

        IbcDispatcher(IBCDispatcher).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );

        return keccak256(data);
    }

    function onAcknowledgementPacket(IbcPacket calldata packet) external {
        bytes calldata data = packet.data;

        /**
         *   Context Flag
         *      2^0: Asset (0) or Liquidity swap (1)
         *        1: Non-approximate (0) or approximate (1)
         *        2: Unused
         *        3: Unused
         *        .
         *        .
         *        7: Unused
         */
        // require(validArriver[msg.sender], "Polymerase Only");  // dev: Only callable by Polymerase

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
        */
        bytes1 _context = data[0];
        address fromPool = abi.decode(data[1:33], (address));
        uint256 U = uint256(bytes32(data[97:129]));

        if ((_context & 0x01) == 0) {
            /* 
                97-128 _U : uint265
            */
            ICatalystV1Pool(fromPool).releaseEscrowACK(
                keccak256(data),
                U,
                uint256(bytes32(data[162:194])), // escrowAmount
                abi.decode(data[194:226], (address)) // escrowToken
            );
        } else {
            ICatalystV1Pool(fromPool).releaseLiquidityEscrowACK(
                keccak256(data),
                U,
                uint256(bytes32(data[161:193])) // escrowAmount
            );
        }
    }

    function onTimeoutPacket(IbcPacket calldata packet) external {
        bytes calldata data = packet.data; // Specificing the location of the data.
        /**
         * Context Flag
         *      2^0: Asset (0) or Liquidity swap (1)
         *        1: Non-approximate (0) or approximate (1)
         *        2: Unused
         *        3: Unused
         *        .
         *        .
         *        7: Unused
         */
        // require(validArriver[msg.sender], "Polymerase Only");  // dev: Only callable by Polymerase

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
        */
        bytes1 _context = data[0]; // Only reverts if the data is truely empty. Well formed data is not.
        address fromPool = abi.decode(data[1:33], (address)); // ^^ Data is well formed.
        uint256 U = uint256(bytes32(data[97:129]));

        if ((_context & 0x01) == 0) {
            // Always true if token swaps.

            ICatalystV1Pool(fromPool).releaseEscrowTIMEOUT(
                keccak256(data), // ^^ Data is well formed.
                U,
                uint256(bytes32(data[162:194])), // escrowAmount
                abi.decode(data[194:226], (address)) // escrowToken
            );
        } else {
            ICatalystV1Pool(fromPool).releaseLiquidityEscrowTIMEOUT(
                keccak256(data), // ^^ Data is well formed.
                U,
                uint256(bytes32(data[161:193])) // escrowAmount
            );
        } // Liquidity swaps.
    }

    function onRecvPacket(IbcPacket calldata packet) external {
        bytes calldata data = packet.data;
        /* 
            Context indicator
                0x00: Ordinary
                0x01: ApproxFromUnits
                0x02: LiquiditySwap
                0x03: ApproxFromLiquidityUnits
         */
        // require(validArriver[msg.sender]); // dev: Only callable by Polymerase
        // string memory channelid = packet.src.channelId;

        /* 
            0 _context : Bytes[1]
            1-32 _fromPool : bytes32
            33-64 _pool : bytes32
        */
        bytes1 _context = data[0];
        address pool = abi.decode(data[33:65], (address));
        // bytes32 messageHash = keccak256(data);

        {
            /* bytes32 fromPool = bytes32(data[1:33]);
        // require(
        //     checkConnection[channelid][bytes32(data[33:65])][fromPool],
        //     NO_CONNECTION
        // ); */
        }

        if ((_context & 0x01) == 0) {
            /* 
                0 _context : Bytes[1]
                1-32 _fromPool : bytes32
                33-64 _pool : bytes32
                65-96 _targetUser : bytes32
                97-128 _U : uint265
                129 _assetIndex : uint8
                130-161 _minOut : uint256
                162-193 _escrowAmount : uint256
                194-214 _escrowToken
                215-216 _customDataLength : uint16  # If custom data is passed.
                217-248+_customDataLength-1 _customData : bytes...
            */
            // uint8 assetIndex = uint8(data[129]);
            // address who = abi.decode(data[65:97], (address));
            // uint256 U = uint256(bytes32(data[97:129]));
            // uint256 minOut = uint256(bytes32(data[130:162]));
            uint16 customDataLength = uint16(bytes2(data[226:228]));

            if (customDataLength > 0) {
                address callDataTarget = abi.decode(data[228:260], (address));
                bytes memory calldata_ = data[260:260 + customDataLength - 32];
                ICatalystV1Pool(pool).swapFromUnits(
                    uint8(data[129]), // assetIndex
                    abi.decode(data[65:97], (address)), // who
                    uint256(bytes32(data[97:129])), // U
                    uint256(bytes32(data[130:162])), // minOut
                    keccak256(data), // messageHash
                    callDataTarget,
                    calldata_
                );
                return;
            }
            ICatalystV1Pool(pool).swapFromUnits(
                uint8(data[129]), // assetIndex
                abi.decode(data[65:97], (address)), // who
                uint256(bytes32(data[97:129])), // U
                uint256(bytes32(data[130:162])), // minOut
                keccak256(data) // messageHash
            );
        } else {
            address who = abi.decode(data[65:97], (address));
            uint256 U = uint256(bytes32(data[97:129]));
            uint256 minOut = uint256(bytes32(data[129:161]));

            ICatalystV1Pool(pool).inLiquidity(
                who,
                U,
                minOut,
                keccak256(data)
            );
        }
    }
}
