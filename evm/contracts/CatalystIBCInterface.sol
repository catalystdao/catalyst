//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "./polymerase/IbcDispatcher.sol";
import "./polymerase/IbcReceiver.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./ICatalystV1Pool.sol";
import "./interfaces/ICatalystV1PoolState.sol"; // structs
import "./CatalystIBCPayload.sol";


/**
 * @title Catalyst: Generalised IBC Interface
 * @author Catalyst Labs
 * @notice This contract is a generalised proof of concept
 * IBC interface using an example ABI.
 * It acts as an intermediate between the swap pool and the router to
 * abstract router logic away from the swap pools. This simplifies the
 * development of the swap pools and allows Catalyst to adopt or change
 * message routers with more flexibility.
 */
contract CatalystIBCInterface is Ownable, IbcReceiver {
    //--- ERRORS ---//
    error InvalidIBCCaller(address caller);  // Only the message router should be able to deliver messages.
    error InvalidContext(bytes1 context);

    //--- Config ---//
    uint256 constant MAXIMUM_TIME_FOR_TX = 2 hours;
    address public immutable IBC_DISPATCHER; // Set on deployment

    constructor(address IBCDispatcher_) {
        require(IBCDispatcher_ != address(0));  // dev: Provided IBCDispatcher_ address cannot be zero
        IBC_DISPATCHER = IBCDispatcher_;
    }

    /// @notice Registers IBC ports for this contract.
    /// @dev The matching CatalystIBCInterface should call
    /// registerPort at the same time to establish an
    /// IBC connection between the 2 contracts.
    function registerPort() external onlyOwner {
        IbcDispatcher(IBC_DISPATCHER).registerPort();
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target pool with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy pool.
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in bytes32.
     * @param toAccount recipient of the transaction on the target chain. Encoded in bytes32.
     * @param toAssetIndex The index of the asset the user wants to buy in the target pool.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param metadata Metadata on the asset swap, used for swap identification and ack/timeout.
     * @param calldata_ Data field if a call should be made on the target chain. 
     * Should be encoded abi.encode(<address>,<data>)
     */
    function sendCrossChainAsset(
        bytes32 channelId,
        bytes calldata toPool,
        bytes calldata toAccount,
        uint8 toAssetIndex,
        uint256 U,
        uint256 minOut,
        AssetSwapMetadata calldata metadata,
        bytes calldata calldata_
    ) external {
        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystIBCPayload.sol for the payload definition
        bytes memory data = bytes.concat(  // Using bytes.concat to circumvent stack too deep error
            abi.encodePacked(
                CTX0_ASSET_SWAP,
                uint8(20),  // EVM addresses are 20 bytes.
                bytes32(0),  // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(msg.sender),  // Use abi.encode to encode address into 32 bytes
                toPool,  // Length is expected to be pre-encoded.
                toAccount,  // Length is expected to be pre-encoded.
                U,
                toAssetIndex,
                minOut
            ),
            abi.encodePacked(
                metadata.fromAmount,
                uint8(20),  // EVM addresses are 20 bytes.
                bytes32(0),  // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(metadata.fromAsset),
                metadata.blockNumber,
                metadata.swapHash,
                uint16(calldata_.length),
                calldata_
            )
        );

        IbcDispatcher(IBC_DISPATCHER).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );

    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target pool with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy pool.
     * @param channelId The target chain identifier.
     * @param toPool The target pool on the target chain encoded in bytes32.
     * @param toAccount recipient of the transaction on the target chain. Encoded in bytes32.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut The minimum number of returned tokens to the toAccount on the target chain.
     * @param metadata Metadata on the asset swap, used for swap identification and ack/timeout.
     * @param calldata_ Data field if a call should be made on the target chain. 
     * Should be encoded abi.encode(<address>,<data>)
     */
    function sendCrossChainLiquidity(
        bytes32 channelId,
        bytes calldata toPool,
        bytes calldata toAccount,
        uint256 U,
        uint256 minOut,
        LiquiditySwapMetadata calldata metadata,
        bytes calldata calldata_
    ) external {
        require(toPool.length == 64);  // dev: External addresses needs to be of length 64
        require(toAccount.length == 64);  // dev: External addresses needs to be of length 64
        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystIBCPayload.sol for the payload definition
        bytes memory data = bytes.concat(  // Using bytes.concat to circumvent stack too deep error
            abi.encodePacked(
                CTX1_LIQUIDITY_SWAP,
                uint8(20),  // EVM addresses are 20 bytes.
                bytes32(0),  // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(msg.sender),  // Use abi.encode to encode address into 32 bytes
                toPool,  // Length is expected to be pre-encoded.
                toAccount,  // Length is expected to be pre-encoded.
                U,
                minOut
            ),
            abi.encodePacked(
                metadata.fromAmount,
                metadata.blockNumber,
                metadata.swapHash,
                uint16(calldata_.length),
                calldata_
            )
        );

        IbcDispatcher(IBC_DISPATCHER).sendIbcPacket(
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
        if (msg.sender != IBC_DISPATCHER) revert InvalidIBCCaller(msg.sender);

        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromPool must be an EVM address.
        address fromPool = address(uint160(bytes20(data[ FROM_POOL_START_EVM : FROM_POOL_END ])));

        if (context == CTX0_ASSET_SWAP) {

            ICatalystV1Pool(fromPool).sendAssetAck(
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                            // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                              // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),        // fromAmouant
                address(uint160(bytes20(data[ CTX0_FROM_ASSET_START_EVM : CTX0_FROM_ASSET_END ]))), // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))         // block number
            );

        }
        else if (context == CTX1_LIQUIDITY_SWAP) {

            ICatalystV1Pool(fromPool).sendLiquidityAck(
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                        // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );

        }
        else {

            revert InvalidContext(context);

        }

    }

    /**
     * @notice IBC timeout package handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function onTimeoutPacket(IbcPacket calldata packet) external {
        if (msg.sender != IBC_DISPATCHER) revert InvalidIBCCaller(msg.sender);

        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromPool must be an EVM address.
        address fromPool = address(uint160(bytes20(data[ FROM_POOL_START_EVM : FROM_POOL_END ])));

        if (context == CTX0_ASSET_SWAP) {

            ICatalystV1Pool(fromPool).sendAssetTimeout(
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                        // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                              // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),        // fromAmount
                abi.decode(data[ CTX0_FROM_ASSET_START : CTX0_FROM_ASSET_END ], (address)),     // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))         // block number
            );

        }
        else if (context == CTX1_LIQUIDITY_SWAP) {

            ICatalystV1Pool(fromPool).sendLiquidityTimeout(
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                        // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );

        }
        else {

            revert InvalidContext(context);

        }
        
    }

    /**
     * @notice IBC package handler
     * @param packet The IBC packet
     */
    function onRecvPacket(IbcPacket calldata packet) external {
        if (msg.sender != IBC_DISPATCHER) revert InvalidIBCCaller(msg.sender);

        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        bytes calldata fromPool = data[ FROM_POOL_LENGTH_POS : FROM_POOL_END ];
        // We know that toPool is an EVM address
        address toPool = address(uint160(bytes20(data[ TO_POOL_START_EVM : TO_POOL_END ])));

        if (context == CTX0_ASSET_SWAP) {

            uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));

            // CCI sets dataLength > 0 if calldata is passed.
            if (dataLength != 0) {
                ICatalystV1Pool(toPool).receiveAsset(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromPool,                                                                   // fromPool
                    uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                       // toAssetIndex
                    address(uint160(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]))),   // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),            // minOut
                    bytes32(data[ CTX0_SWAP_HASH_START : CTX0_SWAP_HASH_END ]),                 // swapHash
                    address(uint160(bytes20(data[ CTX0_DATA_START : CTX0_DATA_START+20 ]))),    // dataTarget
                    data[ CTX0_DATA_START+20 : CTX0_DATA_START+dataLength-20 ]                  // dataArguments
                );
            } else {
                ICatalystV1Pool(toPool).receiveAsset(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromPool,                                                                   // fromPool
                    uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                       // toAssetIndex
                    address(uint160(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]))),   // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),            // minOut
                    bytes32(data[ CTX0_SWAP_HASH_START : CTX0_SWAP_HASH_END ])                  // swapHash
                );
            }

        }
        else if (context == CTX1_LIQUIDITY_SWAP) {

            uint16 dataLength = uint16(bytes2(data[CTX1_DATA_LENGTH_START:CTX1_DATA_LENGTH_END]));

            // CCI sets dataLength > 0 if calldata is passed.
            if (dataLength != 0) {
                ICatalystV1Pool(toPool).receiveLiquidity(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromPool,                                                                   // fromPool
                    address(uint160(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]))),   // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX1_MIN_OUT_START : CTX1_MIN_OUT_END ])),            // minOut
                    bytes32(data[ CTX1_SWAP_HASH_START : CTX1_SWAP_HASH_END ]),                 // swapHash
                    address(uint160(bytes20(data[ CTX1_DATA_START : CTX1_DATA_START+20 ]))),    // dataTarget
                    data[ CTX1_DATA_START+20 : CTX1_DATA_START+dataLength-20 ]                  // dataArguments
                );
            } else {
                ICatalystV1Pool(toPool).receiveLiquidity(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromPool,                                                                   // fromPool
                    address(uint160(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]))),   // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX1_MIN_OUT_START : CTX1_MIN_OUT_END ])),            // minOut
                    bytes32(data[ CTX1_SWAP_HASH_START : CTX1_SWAP_HASH_END ])                  // swapHash
                );
            }

        }
        else {

            revert InvalidContext(context);

        }

    }
}
