//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import "./external/vIBC/IbcDispatcher.sol";
import "./external/vIBC/IbcReceiver.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./ICatalystV1Vault.sol";
import "./interfaces/ICatalystV1VaultState.sol"; // structs
import "./CatalystPayload.sol";


/**
 * @title Catalyst: Generalised IBC Interface
 * @author Cata Labs
 * @notice This contract is a generalised proof of concept
 * IBC interface using an example ABI.
 * It acts as an intermediate between the vault and the router to
 * abstract router logic away from the vaults. This simplifies the
 * development of the vaults and allows Catalyst to adopt or change
 * message routers with more flexibility.
 */
contract CatalystIBCInterface is Ownable, IbcReceiver {
    //--- ERRORS ---//
    error InvalidIBCCaller(address caller);  // Only the message router should be able to deliver messages.
    error InvalidContext(bytes1 context);
    error InvalidAddress();
    error SubcallOutOfGas();

    //--- Config ---//
    uint256 constant MAXIMUM_TIME_FOR_TX = 2 hours; // Depends on the message router implementation. Some should be longer and others should be shorter.
    address public immutable IBC_DISPATCHER; // Set on deployment

    constructor(address IBCDispatcher_) {
        require(IBCDispatcher_ != address(0));  // dev: Provided IBCDispatcher_ address cannot be zero
        IBC_DISPATCHER = IBCDispatcher_;
    }

    modifier onlyIbcDispatcher() {
        if (msg.sender != IBC_DISPATCHER) revert InvalidIBCCaller(msg.sender);
        _;
    }

    /// @notice Registers IBC ports for this contract.
    /// @dev The matching CatalystIBCInterface should call
    /// registerPort at the same time to establish an
    /// IBC connection between the 2 contracts.
    function registerPort() external onlyOwner {
        IbcDispatcher(IBC_DISPATCHER).registerPort();
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target vault with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy vault.
     * @param channelId The target chain identifier.
     * @param toVault The target vault on the target chain. Encoded in 64 + 1 bytes.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in 64 + 1 bytes.
     * @param toAssetIndex The index of the asset the user wants to buy in the target vault.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut The minimum number output of tokens on the target chain.
     * @param fromAmount Escrow related value. The amount returned if the swap fails.
     * @param fromAsset Escrow related value. The asset that was sold.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: abi.encodePacket(bytes20(<address>), <data>).
     */
    function sendCrossChainAsset(
        bytes32 channelId,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint8 toAssetIndex,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        address fromAsset,
        bytes calldata calldata_
    ) external {
        // We need to ensure that all information is in the correct places. This ensures that calls to this contract
        // will always be decoded semi-correctly even if the input is very incorrect. This also checks that the user 
        // inputs into the swap contracts are correct while making the cross-chain interface flexible for future implementations.
        require(toVault.length == 65);      // dev: External addresses needs to be of length 64 + 1.
        require(toAccount.length == 65);    // dev: External addresses needs to be of length 64 + 1.
        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystPayload.sol for the payload definition
        bytes memory data = abi.encodePacked(
            CTX0_ASSET_SWAP,
            uint8(20),      // EVM addresses are 20 bytes.
            bytes32(0),     // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
            abi.encode(msg.sender),  // Use abi.encode to encode address into 32 bytes
            toVault,    // Length is expected to be pre-encoded.
            toAccount,  // Length is expected to be pre-encoded.
            U,
            toAssetIndex,
            minOut,
            fromAmount,
            uint8(20),      // EVM addresses are 20 bytes.
            bytes32(0),     // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
            abi.encode(fromAsset),  // Use abi.encode to encode address into 32 bytes
            uint32(block.number),   // This is the same as block.number mod 2**32-1
            uint16(calldata_.length),   // max length of calldata is 2**16-1 = 65535 bytes which should be more than plenty.
            calldata_
        );

        IbcDispatcher(IBC_DISPATCHER).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );

    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target vault with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy vault.
     * @param channelId The target chain identifier. 
     * @param toVault The target vault on the target chain. Encoded in 64 + 1 bytes.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in 64 + 1 bytes.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut An array of minout describing: [the minimum number of vault tokens, the minimum number of reference assets]
     * @param fromAmount Escrow related value. The amount returned if the swap fails.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: abi.encodePacket(bytes20(<address>), <data>).
     */
    function sendCrossChainLiquidity(
        bytes32 channelId,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint256 U,
        uint256[2] calldata minOut,
        uint256 fromAmount,
        bytes memory calldata_
    ) external {
        // We need to ensure that all information is in the correct places. This ensures that calls to this contract
        // will always be decoded semi-correctly even if the input is very incorrect. This also checks that the user 
        // inputs into the swap contracts are correct while making the cross-chain interface flexible for future implementations.
        require(toVault.length == 65);      // dev: External addresses needs to be of length 64 + 1.
        require(toAccount.length == 65);    // dev: External addresses needs to be of length 64 + 1.
        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystPayload.sol for the payload definition
        bytes memory data =  abi.encodePacked(
            CTX1_LIQUIDITY_SWAP,
            uint8(20),  // EVM addresses are 20 bytes.
            bytes32(0),  // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
            abi.encode(msg.sender),  // Use abi.encode to encode address into 32 bytes
            toVault,  // Length is expected to be pre-encoded.
            toAccount,  // Length is expected to be pre-encoded.
            U,
            minOut[0],
            minOut[1],
            fromAmount,
            uint32(block.number),
            uint16(calldata_.length),
            calldata_
        );

        IbcDispatcher(IBC_DISPATCHER).sendIbcPacket(
            channelId,
            data,
            uint64(block.timestamp + MAXIMUM_TIME_FOR_TX)
        );
    }

    /**
     * @notice Cross-chain message success handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function _onPacketSuccess(IbcPacket calldata packet) internal {
        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromVault must be an EVM address.
        address fromVault = address(bytes20(data[ FROM_VAULT_START_EVM : FROM_VAULT_END ]));

        if (context == CTX0_ASSET_SWAP) {
            ICatalystV1Vault(fromVault).onSendAssetSuccess(
                bytes32(packet.dest.channelId),                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                address(bytes20(data[ CTX0_FROM_ASSET_START_EVM : CTX0_FROM_ASSET_END ])),  // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // block number
            );
        }
        else if (context == CTX1_LIQUIDITY_SWAP) {
            ICatalystV1Vault(fromVault).onSendLiquiditySuccess(
                bytes32(packet.dest.channelId),                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
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
     * @notice Cross-chain message failure handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function _onPacketFailure(IbcPacket calldata packet) internal {
        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromVault must be an EVM address.
        address fromVault = address(bytes20(data[ FROM_VAULT_START_EVM : FROM_VAULT_END ]));

        if (context == CTX0_ASSET_SWAP) {
            ICatalystV1Vault(fromVault).onSendAssetFailure(
                bytes32(packet.dest.channelId),                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                address(bytes20(data[ CTX0_FROM_ASSET_START_EVM : CTX0_FROM_ASSET_END ])),  // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // block number
            );
        }
        else if (context == CTX1_LIQUIDITY_SWAP) {
            ICatalystV1Vault(fromVault).onSendLiquidityFailure(
                bytes32(packet.dest.channelId),                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
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
     * @notice IBC Acknowledgement package handler
     * @dev Should never revert.
     * @param packet The IBC packet
     * @param acknowledgement The acknowledgement bytes for the cross-chain swap.
     */
    function onAcknowledgementPacket(bytes calldata acknowledgement, IbcPacket calldata packet) onlyIbcDispatcher external {
        // * This contract should also verify the IBC port. (Or the sending contract of the IBC packet)
        // If the transaction executed but some logic failed, an ack is sent back with an acknowledgement of not 0x00.
        // This is known as "fail on ack". The package should be timed-out.
        if (acknowledgement[0] != 0x00) return _onPacketFailure(packet);
        // Otherwise, it must be a success:
        _onPacketSuccess(packet);
    }

    /**
     * @notice IBC timeout package handler
     * @dev Should never revert.
     * @param packet The IBC packet
     */
    function onTimeoutPacket(IbcPacket calldata packet) onlyIbcDispatcher external {
        // * This contract should also verify the IBC port. (Or the sending contract of the IBC packet)
        // Timeouts always implies failure.
        _onPacketFailure(packet);
    }

    /**
     * @notice IBC packet handler
     * @param packet The IBC packet
     * @return acknowledgement The acknowledgement status of the transaction after execution
     */
    function onRecvPacket(IbcPacket calldata packet) onlyIbcDispatcher external returns (bytes memory) {
        // * This contract should also verify the IBC port. (Or the sending contract of the IBC packet)
        bytes calldata data = packet.data;

        bytes1 context = data[CONTEXT_POS];
        // We don't know how from_vaul is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        // Check that toAccount is the correct length and only contains 0 bytes beyond the address.
        if (uint8(data[TO_ACCOUNT_LENGTH_POS]) != 20) revert InvalidAddress();  // Check correct length
        if (uint256(bytes32(data[TO_ACCOUNT_START:TO_ACCOUNT_START+32])) != 0) revert InvalidAddress();  // Check first 32 bytes are 0.
        if (uint96(bytes12(data[TO_ACCOUNT_START+32:TO_ACCOUNT_START_EVM])) != 0) revert InvalidAddress();  // Check the next 32-20=12 bytes are 0.
        // To vault will not be checked. If it is assumed that any error is random, then an incorrect toVault will result in the call failling.

        bytes1 acknowledgement = 0x01; // Default status of a transaction is failed.
        if (context == CTX0_ASSET_SWAP) {
            uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));
            // CCI sets dataLength > 0 if calldata is passed.
            if (dataLength != 0) {
                try ICatalystV1Vault(toVault).receiveAsset(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromVault,                                                                   // fromVault
                    uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                       // toAssetIndex
                    address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),           // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),            // minOut
                    uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                    bytes(data[ CTX0_FROM_ASSET_LENGTH_POS : CTX0_FROM_ASSET_END ]),            // fromAsset
                    uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ])),    // block number
                    address(bytes20(data[ CTX0_DATA_START : CTX0_DATA_START+20 ])),            // dataTarget
                    data[ CTX0_DATA_START+20 : CTX0_DATA_START+dataLength ]                     // dataArguments
                ) {acknowledgement = 0x00;} catch (bytes memory err) {
                    // Since the majoirty gas cost of this call is within the try catch statement (receiveAsset),
                    // a cheating relayer could pay enough gas where the call would run out of gas but the rest of the logic
                    // would still execute. There is no reliable way to catch that here.
                    // One solution would be to try to catch empty bytes but some errors also revert with this.
                    // We need to deal with this later.
                }
            } else {
                try ICatalystV1Vault(toVault).receiveAsset(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromVault,                                                                   // fromVault
                    uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                       // toAssetIndex
                    address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),           // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),            // minOut
                    uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                    bytes(data[ CTX0_FROM_ASSET_LENGTH_POS : CTX0_FROM_ASSET_END ]),            // fromAsset
                    uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // blocknumber
                ) {acknowledgement = 0x00;} catch (bytes memory err) {
                    // See previous note about 'Out of Gas'
                }
            }
        }
        else if (context == CTX1_LIQUIDITY_SWAP) {

            uint16 dataLength = uint16(bytes2(data[CTX1_DATA_LENGTH_START:CTX1_DATA_LENGTH_END]));

            // CCI sets dataLength > 0 if calldata is passed.
            if (dataLength != 0) {
                try ICatalystV1Vault(toVault).receiveLiquidity(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromVault,                                                                  // fromVault
                    address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),            // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX1_MIN_VAULT_TOKEN_START : CTX1_MIN_VAULT_TOKEN_END ])), // minOut
                    uint256(bytes32(data[ CTX1_MIN_REFERENCE_START : CTX1_MIN_REFERENCE_END ])),// minOut
                    uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                    uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ])),    // block number
                    address(bytes20(data[ CTX1_DATA_START : CTX1_DATA_START+20 ])),             // dataTarget
                    data[ CTX1_DATA_START+20 : CTX1_DATA_START+dataLength ]                     // dataArguments
                ) {acknowledgement = 0x00;} catch (bytes memory err) {
                    // See previous note about 'Out of Gas'
                }
            } else {
                try ICatalystV1Vault(toVault).receiveLiquidity(
                    bytes32(packet.src.channelId),                                              // connectionId
                    fromVault,                                                                   // fromVault
                    address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),            // toAccount
                    uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                    uint256(bytes32(data[ CTX1_MIN_VAULT_TOKEN_START : CTX1_MIN_VAULT_TOKEN_END ])), // minOut
                    uint256(bytes32(data[ CTX1_MIN_REFERENCE_START : CTX1_MIN_REFERENCE_END ])),// minOut
                    uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                    uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // blocknumber
                ) {acknowledgement = 0x00;} catch (bytes memory err) {
                    // See previous note about 'Out of Gas'
                }
            }
        }
        else {
            /* revert InvalidContext(context); */
            acknowledgement = 0x02;
        }

         // To ensure relayers provide enough gas, we need to make sure that the transaction didn't
        // fail because too little gas was provided. It is not possible to place a check inside the try-catch
        // since it will always be unable to catch everything. The safe-erc20 lib used also has a try/catch style check.
        // If that call runs out of gas, the error is TRANSFER_FAILED which wouldn't match an ordinary out-of-gas check and
        // sometimes wants to catch that.
        // The "solution" is to check if there is more than 21000 gas left. While hardcoding a gas limit is not great it works pretty well. 
        // The biggest issue with this implementation is that only 63/64 of the gas is sent along with the call. If changes are made 
        // to the gas pricing such that the gas usage increases where 1/64'th of the gas is enough to execute the rest of the logic AND
        // leave 21000 gas left, then this will never activate. That is, the swap needs to spend at least 63*21000 = 1 323 000 gas.
        // Currently the gas cost is around ~ 124000 gas. That implies that the protection stops working if the EVM implementation
        // increases the gas cost of the call by 10x.
        if (gasleft() < 21000) revert SubcallOutOfGas();
        return abi.encodePacked(acknowledgement);
    }
}
