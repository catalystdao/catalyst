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
        // but unless someone can also manage to pass the security check on onRecvPacket
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


        // Encode payload. See CatalystIBCPayload.sol for the payload definition

        // To limit the number of active variables on the stack
        // calldata is encoded separately.
        bytes memory preparedCalldata = abi.encodePacked(
            uint16(calldata_.length),
            calldata_
        );

        // To limit the number of active variables on the stack
        // escrow information is encoded separately.
        bytes memory preparedEscrowAndCalldata = abi.encodePacked(
            escrowInformation.amount,
            abi.encode(escrowInformation.token),
            uint32(block.number % 2**32),
            escrowInformation.swapHash,
            preparedCalldata
        );

        // abi.encode always encodes to 32 bytes.
        // abi.encodePacked encodes in the smallest possible bytes.
        // 32 bytes are reserved for addresses (for chain compatibility)..
        // Ethereum addresses only use 20 bytes, so abi.encodePacked returns 20 bytes whereas abi.encode returns 32 bytes.
        // We want 32 just in case other chains use 32 bytes ids.
        // abi.encodePacked encodes the arguments as a concat.
        bytes memory data = abi.encodePacked(
            CTX0_ASSET_SWAP,
            abi.encode(msg.sender),
            targetPool,
            targetUser,
            U,
            uint8(targetAssetIndex),
            minOut,
            preparedEscrowAndCalldata
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
        // but unless someone can also manage to pass the security check on onRecvPacket
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

        // Encode payload. See CatalystIBCPayload.sol for the payload definition

        // abi.encode always encodes to 32 bytes.
        // abi.encodePacked encodes in the smallest possible bytes.
        // 32 bytes are reserved for addresses (for chain compatibility)..
        // Ethereum addresses only use 20 bytes, so abi.encodePacked returns 20 bytes whereas abi.encode returns 32 bytes.
        // We want 32 just in case other chains use 32 bytes ids.
        // abi.encodePacked encodes the arguments as a concat.
        bytes memory data = abi.encodePacked(
            CTX1_LIQUIDITY_SWAP,
            abi.encode(msg.sender),
            targetPool,
            targetUser,
            U,
            minOut,
            escrowInformation.poolTokens,
            uint32(block.number % 2**32),
            escrowInformation.swapHash,
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
        bytes calldata data = packet.data;
        // TODO: Enable
        // require(IBCDispatcher == msg.sender, ONLY_IBC_CALLER);

        bytes1 context = data[CONTEXT_POS];
        address fromPool = abi.decode(data[ FROM_POOL_START : FROM_POOL_END ], (address));

        // Check if the flag 0x01 is set. If it is, it is a liquidity swap.
        if (context == CTX1_LIQUIDITY_SWAP) {
            // Delete the escrow information for liquidity swaps.
            ICatalystV1Pool(fromPool).sendLiquidityAck(
                bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                         // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );
            return;
        } 
        // Otherwise, it is an ordinary swap.

        // Delete the escrow information for ordinary swaps.
        ICatalystV1Pool(fromPool).sendSwapAck(
            bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                             // toAccount
            uint256(bytes32(data[ UNITS_START : UNITS_END ])),                              // units
            uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),        // fromAmount
            abi.decode(data[ CTX0_FROM_ASSET_START : CTX0_FROM_ASSET_END ], (address)),     // fromAsset
            uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))         // block number
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

        bytes1 context = data[CONTEXT_POS];
        address fromPool = abi.decode(data[FROM_POOL_START:FROM_POOL_END], (address));

        if (context == CTX1_LIQUIDITY_SWAP) {
            // Release the liquidiy escrow.
            ICatalystV1Pool(fromPool).sendLiquidityTimeout(
                bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                         // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );
            return;
        }

        // Release the ordinary escrow.
        ICatalystV1Pool(fromPool).sendSwapTimeout(
            bytes32(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ]),                             // toAccount
            uint256(bytes32(data[ UNITS_START : UNITS_END ])),                              // units
            uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),        // fromAmount
            abi.decode(data[ CTX0_FROM_ASSET_START : CTX0_FROM_ASSET_END ], (address)),     // fromAsset
            uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))         // block number
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

        bytes1 context = data[CONTEXT_POS];
        bytes32 fromPool = bytes32(data[ FROM_POOL_START : FROM_POOL_END ]);
        address toPool = abi.decode(data[ TO_POOL_START : TO_POOL_END ], (address));

        require(
            checkConnection[bytes32(packet.src.channelId)][bytes32(data[ TO_POOL_START : TO_POOL_END ])][fromPool],
            NO_CONNECTION
        );

        // Check if the swap is a liquidity swap.
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
        // It is not, so it is an asset swap.

        uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));

        // CCI sets dataLength > 0 if calldata is passed.
        if (dataLength != 0) {
            ICatalystV1Pool(toPool).receiveSwap(
                fromPool,                                                            // sourcePool
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
            fromPool,                                                                // sourcePool
            uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                    // toAssetIndex
            abi.decode(data[ TO_ACCOUNT_START : TO_ACCOUNT_END ], (address)),        // toAccount
            uint256(bytes32(data[ UNITS_START : UNITS_END ])),                       // units
            uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),         // minOut
            bytes32(data[ CTX0_SWAP_HASH_START : CTX0_SWAP_HASH_END ])               // swapHash
        );

    }
}
