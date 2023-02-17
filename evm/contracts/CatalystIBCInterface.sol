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
     * @param channelId The chain identifier to modify the pools connect with.
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
    function crossChainSwap(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        uint8 toAssetIndex,
        uint256 U,
        uint256 minOut,
        AssetSwapMetadata memory metadata,
        bytes memory calldata_
    ) external {
        // Anyone can call this function. And anyone can pass the security check later,
        // but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value.
        // As such, the very worst they can do is waste gas.

        // Catch bad messages early. If there is no connection,
        // don't send a message.
        // Encode the sender in bytes32.
        require(
            checkConnection[channelId][bytes32(abi.encode(msg.sender))][toPool],
            NO_CONNECTION
        );


        // Encode payload. See CatalystIBCPayload.sol for the payload definition
        bytes memory data = bytes.concat(       // Using bytes.concat to circumvent stack too deep error
            abi.encodePacked(
                CTX0_ASSET_SWAP,
                abi.encode(msg.sender),         // Use abi.encode to encode address into 32 bytes
                toPool,
                toAccount,
                U,
                toAssetIndex,
                minOut
            ),
            abi.encodePacked(
                metadata.fromAmount,
                abi.encode(metadata.fromAsset),
                metadata.blockNumber,
                metadata.swapHash,
                uint16(calldata_.length),
                calldata_
            )
        );

        IbcDispatcher(IBCDispatcher).sendIbcPacket(
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
     */
    function liquiditySwap(
        bytes32 channelId,
        bytes32 toPool,
        bytes32 toAccount,
        uint256 U,
        uint256 minOut,
        LiquiditySwapMetadata memory metadata
    ) external {
        // Anyone can call this function. And anyone can pass the security check later,
        // but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value.
        // As such, the very worst they can do is waste gas.

        // Catch bad messages early. If there is no connection,
        // don't send a message.
        // Encode the sender in bytes32.
        require(
            checkConnection[channelId][bytes32(abi.encode(msg.sender))][toPool],
            NO_CONNECTION
        );

        // Encode payload. See CatalystIBCPayload.sol for the payload definition
        bytes memory data = abi.encodePacked(
            CTX1_LIQUIDITY_SWAP,
            abi.encode(msg.sender),         // Use abi.encode to encode address into 32 bytes
            toPool,
            toAccount,
            U,
            minOut,
            metadata.fromAmount,
            metadata.blockNumber,
            metadata.swapHash,
            uint8(0)                        // Set DATA length to 0
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
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        address fromPool = abi.decode(data[ FROM_POOL_START : FROM_POOL_END ], (address));

        if (context == CTX0_ASSET_SWAP) {

            ICatalystV1Pool(fromPool).sendSwapAck(
                bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                              // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),        // fromAmount
                abi.decode(data[ CTX0_FROM_ASSET_START : CTX0_FROM_ASSET_END ], (address)),     // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))         // block number
            );

            return;
        }

        if (context == CTX1_LIQUIDITY_SWAP) {

            ICatalystV1Pool(fromPool).sendLiquidityAck(
                bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                         // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );

            return;
        } 
        
        revert("Undefined Context");

    }

    /**
     * @notice IBC timeout package handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function onTimeoutPacket(IbcPacket calldata packet) external {
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        address fromPool = abi.decode(data[FROM_POOL_START:FROM_POOL_END], (address));

        if (context == CTX0_ASSET_SWAP) {

            ICatalystV1Pool(fromPool).sendSwapTimeout(
                bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                              // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),        // fromAmount
                abi.decode(data[ CTX0_FROM_ASSET_START : CTX0_FROM_ASSET_END ], (address)),     // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))         // block number
            );

            return;
        }

        if (context == CTX1_LIQUIDITY_SWAP) {

            ICatalystV1Pool(fromPool).sendLiquidityTimeout(
                bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                         // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );

            return;
        }
        
        revert("Undefined Context");
        
    }

    /**
     * @notice IBC package handler
     * @param packet The IBC packet
     */
    function onRecvPacket(IbcPacket calldata packet) external {
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        bytes32 fromPool = bytes32(data[ FROM_POOL_START : FROM_POOL_END ]);
        address toPool = abi.decode(data[ TO_POOL_START : TO_POOL_END ], (address));

        require(
            checkConnection[bytes32(packet.src.channelId)][bytes32(data[ TO_POOL_START : TO_POOL_END ])][fromPool],
            NO_CONNECTION
        );


        if (context == CTX0_ASSET_SWAP) {

            uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));

            // CCI sets dataLength > 0 if calldata is passed.
            if (dataLength != 0) {
                ICatalystV1Pool(toPool).receiveSwap(
                    fromPool,                                                            // fromPool
                    uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                // toAssetIndex
                    abi.decode(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ], (address)),    // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                   // units
                    uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),     // minOut
                    bytes32(data[ CTX0_SWAP_HASH_START : CTX0_SWAP_HASH_END ]),          // swapHash
                    abi.decode(data[ CTX0_DATA_START : CTX0_DATA_START+32 ], (address)), // dataTarget
                    data[ CTX0_DATA_START+32 : dataLength-32 ]                           // dataArguments
                );

                return;
            }

            ICatalystV1Pool(toPool).receiveSwap(
                fromPool,                                                                // fromPool
                uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                    // toAssetIndex
                abi.decode(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ], (address)),        // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                       // units
                uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),         // minOut
                bytes32(data[ CTX0_SWAP_HASH_START : CTX0_SWAP_HASH_END ])               // swapHash
            );

            return;

        }

        if (context == CTX1_LIQUIDITY_SWAP) {

            ICatalystV1Pool(toPool).receiveLiquidity(
                fromPool,
                abi.decode(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ], (address)),   // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                  // units
                uint256(bytes32(data[ CTX1_MIN_OUT_START : CTX1_MIN_OUT_END ])),    // minOut
                bytes32(data[ CTX1_SWAP_HASH_START : CTX1_SWAP_HASH_END ])          // swapHash
            );

            return;
        }
        
        revert("Undefined Context");

    }
}
