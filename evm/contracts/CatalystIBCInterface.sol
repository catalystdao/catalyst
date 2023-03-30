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
        AssetSwapMetadata memory metadata,
        bytes memory calldata_
    ) external {
        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystIBCPayload.sol for the payload definition
        bytes memory data = bytes.concat(       // Using bytes.concat to circumvent stack too deep error
            abi.encodePacked(
                CTX0_ASSET_SWAP,
                uint8(20),
                bytes20(uint160(msg.sender)),
                uint8(toPool.length),
                toPool,
                uint8(toAccount.length),
                toAccount,
                U,
                toAssetIndex,
                minOut
            ),
            abi.encodePacked(
                metadata.fromAmount,
                uint8(20),
                bytes20(uint160(metadata.fromAsset)),
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
        LiquiditySwapMetadata memory metadata,
        bytes memory calldata_
    ) external {
        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystIBCPayload.sol for the payload definition
        bytes memory data = bytes.concat(       // Using bytes.concat to circumvent stack too deep error
            abi.encodePacked(
                CTX1_LIQUIDITY_SWAP,
                uint8(20),
                bytes20(uint160(msg.sender)),
                uint8(toPool.length),
                toPool,
                uint8(toAccount.length),
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

        unchecked {
            // All of the additons within this block will be limited to the size of the cross-chain
            // message except the data length which can extend beyond the data length. This implies
            // no addition can overflow.
            // Notice that all from addresses are EVM while all target addresses are unknown.

            // variable_data_length is used to adjust the indexes of the message to compensate for differences in the address size
            // between different implementations.
            uint256 variable_data_length = 0;

            bytes1 context = data[0];  // Context position is always 0

            // Fetch the sending pool length. We know that fromPool is an Ethereum address, so we know the length: 20 bytes.
            // uint256 address_length = uint256(uint8(data[FROM_POOL_LENGTH_POS]));
            address fromPool = address(uint160(bytes20(data[ FROM_POOL_START : FROM_POOL_START + 20 ])));
            variable_data_length += 20;

            // Fetch the target pool length. We don't know anything about the address except that it is less than 256 bytes.
            uint256 address_length = uint256(uint8(data[ TO_POOL_LENGTH_POS + variable_data_length ]));
            variable_data_length += address_length;

            // Fetch the target account. We don't know anything about the address except that it is less than 256 bytes.
            address_length = uint256(uint8(data[ TO_ACCOUNT_POS + variable_data_length ]));
            bytes memory toAccount = bytes(data[ TO_ACCOUNT_START + variable_data_length : TO_ACCOUNT_START + address_length + variable_data_length ]);
            variable_data_length += address_length;

            uint256 units = uint256(bytes32(data[ UNITS_START + variable_data_length : UNITS_END + variable_data_length ]));
        
            if (context == CTX0_ASSET_SWAP) {

                // Fetch the from asset size. We know that from asset is an Ethereum address, so we know the length: 20 bytes.
                address_length = 20;
                address fromAsset = address(uint160(bytes20(data[CTX0_FROM_ASSET_START+variable_data_length:CTX0_FROM_ASSET_START+20+variable_data_length])));
                // The order of variables is now important, as every variable loaded after to account
                // requires taking the size of to account into consideration.

                ICatalystV1Pool(fromPool).sendAssetAck(
                    toAccount,                          // toAccount
                    units,                              // units
                    uint256(bytes32(data[CTX0_FROM_AMOUNT_START+variable_data_length:CTX0_FROM_AMOUNT_END+variable_data_length])),          // fromAmount
                    fromAsset,                          // fromAsset
                    uint32(bytes4(data[CTX0_BLOCK_NUMBER_START+20+variable_data_length:CTX0_BLOCK_NUMBER_END+20+variable_data_length]))     // block number
                );

            }
            else if (context == CTX1_LIQUIDITY_SWAP) {

                ICatalystV1Pool(fromPool).sendLiquidityAck(
                    toAccount,                          // toAccount
                    units,                              // units
                    uint256(bytes32(data[CTX1_FROM_AMOUNT_START+variable_data_length:CTX1_FROM_AMOUNT_END+variable_data_length])),    // fromAmount
                    uint32(bytes4(data[CTX1_BLOCK_NUMBER_START+variable_data_length:CTX1_BLOCK_NUMBER_END+variable_data_length]))     // block number
                );

            }
            else {

                revert InvalidContext(context);

            }
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

        unchecked {
            // All of the additons within this block will be limited to the size of the cross-chain
            // message except the data length which can extend beyond the data length. This implies
            // no addition can overflow.
            // Notice that all from addresses are EVM while all target addresses are unknown.

            // variable_data_length is used to adjust the indexes of the message to compensate for differences in the address size
            // between different implementations.
            uint256 variable_data_length = 0;

            bytes1 context = data[0];  // Context position is always 0

            // Fetch the sending pool length. We know that fromPool is an Ethereum address, so we know the length: 20 bytes.
            // uint256 address_length = uint256(uint8(data[FROM_POOL_LENGTH_POS]));
            address fromPool = address(uint160(bytes20(data[ FROM_POOL_START : FROM_POOL_START + 20 ])));
            variable_data_length += 20;

            // Fetch the target pool length. We don't know anything about the address except that it is less than 256 bytes.
            uint256 address_length = uint256(uint8(data[ TO_POOL_LENGTH_POS + variable_data_length ]));
            variable_data_length += address_length;

            // Fetch the target account. We don't know anything about the address except that it is less than 256 bytes.
            address_length = uint256(uint8(data[ TO_ACCOUNT_POS + variable_data_length ]));
            bytes memory toAccount = bytes(data[ TO_ACCOUNT_START + variable_data_length : TO_ACCOUNT_START + address_length + variable_data_length ]);
            variable_data_length += address_length;

            uint256 units = uint256(bytes32(data[ UNITS_START + variable_data_length : UNITS_END + variable_data_length ]));
        
            if (context == CTX0_ASSET_SWAP) {

                // Fetch the from asset size. We know that from asset is an Ethereum address, so we know the length: 20 bytes.
                // address_length = 20;
                address fromAsset = address(uint160(bytes20(data[CTX0_FROM_ASSET_START+variable_data_length:CTX0_FROM_ASSET_START+20+variable_data_length])));
                // The order of variables is now important, as every variable loaded after to account
                // requires taking the size of to account into consideration.

                ICatalystV1Pool(fromPool).sendAssetTimeout(
                    toAccount,                          // toAccount
                    units,                              // units
                    uint256(bytes32(data[CTX0_FROM_AMOUNT_START+variable_data_length:CTX0_FROM_AMOUNT_END+variable_data_length])),          // fromAmount
                    fromAsset,                          // fromAsset
                    uint32(bytes4(data[CTX0_BLOCK_NUMBER_START+20+variable_data_length:CTX0_BLOCK_NUMBER_END+20+variable_data_length]))     // block number
                );

            }
            else if (context == CTX1_LIQUIDITY_SWAP) {

                ICatalystV1Pool(fromPool).sendLiquidityTimeout(
                    toAccount,                          // toAccount
                    units,                              // units
                    uint256(bytes32(data[CTX1_FROM_AMOUNT_START+variable_data_length:CTX1_FROM_AMOUNT_END+variable_data_length])),    // fromAmount
                    uint32(bytes4(data[CTX1_BLOCK_NUMBER_START+variable_data_length:CTX1_BLOCK_NUMBER_END+variable_data_length]))     // block number
                );

            }
            else {

                revert InvalidContext(context);

            }
        }
        
    }

    /**
     * @notice IBC package handler
     * @param packet The IBC packet
     */
    function onRecvPacket(IbcPacket calldata packet) external {
        if (msg.sender != IBC_DISPATCHER) revert InvalidIBCCaller(msg.sender);

        bytes calldata data = packet.data;

        unchecked {
            // All of the additons within this block will be limited to the size of the cross-chain
            // message except the data length which can extend beyond the data length. This implies
            // no addition can overflow.
            // Notice that all target addresses are EVM while all from addresses are unknown.

            // variable_data_length is used to adjust the indexes of the message to compensate for differences in the address size
            // between different implementations.
            uint256 variable_data_length = 0;

            bytes1 context = data[0];  // Context position is always 0

            // Fetch the sending pool. We doesn't know anything about the address except that it is less than 256 bytes.
            uint256 address_length = uint256(uint8(data[FROM_POOL_LENGTH_POS]));
            // We now know the size of the sending address and we can find the address.
            bytes memory fromPool = bytes(data[ FROM_POOL_START : FROM_POOL_START + address_length ]);
            variable_data_length += address_length;

            // Fetch the target pool. We know that toPool is an Ethereum address, so we know the length: 20 bytes.
            address_length = uint256(uint8(data[ TO_POOL_LENGTH_POS + variable_data_length ]));
            require(address_length == 20);  // dev: Addresses are 20 bytes for EVM.
            address toPool = address(uint160(bytes20(data[ TO_POOL_START + variable_data_length : TO_POOL_START + 20 + variable_data_length ])));
            variable_data_length += 20;

            // Fetch the target account. We know that toAccount is an Ethereum address, so we know the length: 20 bytes.
            address_length = uint256(uint8(data[ TO_ACCOUNT_POS + variable_data_length ]));
            require(address_length == 20);  // dev: Addresses are 20 bytes for EVM.
            address toAccount = address(uint160(bytes20(data[ TO_ACCOUNT_START + variable_data_length : TO_ACCOUNT_START + 20 + variable_data_length ])));
            variable_data_length += 20;

            uint256 units = uint256(bytes32(data[ UNITS_START + variable_data_length : UNITS_END + variable_data_length ]));

            if (context == CTX0_ASSET_SWAP) {

                // Fetch the from asset size.
                address_length = uint256(uint8(data[ CTX0_FROM_ASSET_POS + variable_data_length ]));
                // The order of variables is now important, as every variable loaded after to account
                // requires taking the size of to account into consideration.

                uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START+address_length+variable_data_length:CTX0_DATA_LENGTH_END+address_length+variable_data_length]));

                // CCI sets dataLength > 0 if calldata is passed.
                if (dataLength != 0) {
                    ICatalystV1Pool(toPool).receiveAsset(
                        bytes32(packet.src.channelId),                                                                                  // connectionId
                        fromPool,                                                                                                       // fromPool
                        uint8(data[CTX0_TO_ASSET_INDEX_POS+variable_data_length]),                                                      // toAssetIndex
                        toAccount,                                                                                                      // toAccount
                        units,                                                                                                          // units
                        uint256(bytes32(data[CTX0_MIN_OUT_START+address_length+variable_data_length:CTX0_MIN_OUT_END+address_length+variable_data_length])),            // minOut
                        bytes32(data[CTX0_SWAP_HASH_START+address_length+variable_data_length:CTX0_SWAP_HASH_END+address_length+variable_data_length]),                 // swapHash
                        address(uint160(bytes20(data[CTX0_DATA_START+address_length+variable_data_length:CTX0_DATA_START+20+address_length+variable_data_length]))),    // dataTarget
                        data[CTX0_DATA_START+20+address_length+variable_data_length:CTX0_DATA_START+dataLength-20+address_length+variable_data_length]                  // dataArguments
                    );
                } else {
                    ICatalystV1Pool(toPool).receiveAsset(
                        bytes32(packet.src.channelId),                                                                                  // connectionId
                        fromPool,                                                                                                       // fromPool
                        uint8(data[CTX0_TO_ASSET_INDEX_POS+variable_data_length]),                                                      // toAssetIndex
                        toAccount,                                                                                                      // toAccount
                        units,                                                                                                          // units
                        uint256(bytes32(data[CTX0_MIN_OUT_START+20+variable_data_length:CTX0_MIN_OUT_END+20+variable_data_length])),    // minOut
                        bytes32(data[CTX0_SWAP_HASH_START+20+variable_data_length:CTX0_SWAP_HASH_END+20+variable_data_length])          // swapHash
                    );
                }

            }
            else if (context == CTX1_LIQUIDITY_SWAP) {

                uint16 dataLength = uint16(bytes2(data[CTX1_DATA_LENGTH_START+variable_data_length:CTX1_DATA_LENGTH_END+variable_data_length]));

                // CCI sets dataLength > 0 if calldata is passed.
                if (dataLength != 0) {
                    ICatalystV1Pool(toPool).receiveLiquidity(
                        bytes32(packet.src.channelId),                                          // connectionId
                        fromPool,                                                               // fromPool
                        toAccount,                                                              // toAccount
                        units,                                                                  // units
                        uint256(bytes32(data[CTX1_MIN_OUT_START+variable_data_length:CTX1_MIN_OUT_END+variable_data_length])),          // minOut
                        bytes32(data[CTX1_SWAP_HASH_START+variable_data_length:CTX1_SWAP_HASH_END+variable_data_length]),               // swapHash
                        address(uint160(bytes20(data[CTX1_DATA_START+variable_data_length:CTX1_DATA_START+32+variable_data_length]))),  // dataTarget
                        data[CTX1_DATA_START+32+variable_data_length:CTX1_DATA_START+dataLength-32+variable_data_length]                // dataArguments
                    );
                } else {
                    ICatalystV1Pool(toPool).receiveLiquidity(
                        bytes32(packet.src.channelId),                                          // connectionId
                        fromPool,                                                               // fromPool
                        toAccount,                                                              // toAccount
                        units,                                                                  // units
                        uint256(bytes32(data[CTX1_MIN_OUT_START+variable_data_length:CTX1_MIN_OUT_END+variable_data_length])),          // minOut
                        bytes32(data[CTX1_SWAP_HASH_START+variable_data_length:CTX1_SWAP_HASH_END+variable_data_length])                // swapHash
                    );
                }

            }
            else {

                revert InvalidContext(context);

            }
        }


    }
}
