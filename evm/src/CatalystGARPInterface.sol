//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";
import { IIncentivizedMessageEscrow } from "GeneralisedIncentives/src/interfaces/IIncentivizedMessageEscrow.sol";
import { ICrossChainReceiver } from "GeneralisedIncentives/src/interfaces/ICrossChainReceiver.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "./ICatalystV1Vault.sol";
import "./interfaces/ICatalystV1VaultState.sol"; // structs
import "./CatalystPayload.sol";
import {Bytes65} from "GeneralisedIncentives/src/utils/Bytes65.sol";

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
contract CatalystGARPInterface is Ownable, ICrossChainReceiver, Bytes65, IMessageEscrowStructs {
    //--- ERRORS ---//
    error InvalidCaller();  // Only the message router should be able to deliver messages.
    error InvalidContext(bytes1 context);
    error InvalidAddress();
    error InvalidSourceApplication();
    error SubcallOutOfGas();
    error NotEnoughIncentives();
    error ChainAlreadySetup();

    bytes32 constant KECCACK_OF_NOTHING = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470;


    event SwapFailed(bytes1 error);
    event RemoteImplementationSet(bytes32 chainIdentifier, bytes remoteCCI, bytes remoteGARP);
    
    event MinGasFor(
        bytes32 identifier,
        uint48 minGas
    );

    //--- Config ---//
    IIncentivizedMessageEscrow public immutable GARP; // Set on deployment

    // If not set, then the identifier is abi.encodePacked(uint8(20), bytes32(0), bytes32(msg.sender))
    mapping(bytes32 => bytes) public chainIdentifierToDestinationAddress;

    mapping(bytes32 => uint48) public minGasFor;

    constructor(address GARP_) {
        require(address(GARP_) != address(0));  // dev: GARP_ cannot be zero address
        GARP = IIncentivizedMessageEscrow(GARP_);
    }

    /// @notice Allow updating of the minimum gas limit.
    /// @dev Set chainIdentifier to 0 for gas for ack. 
    function setMinGasFor(bytes32 chainIdentifier, uint48 minGas) external onlyOwner {
        minGasFor[chainIdentifier] = minGas;

        emit MinGasFor(chainIdentifier, minGas);
    }

    modifier checkIncentives(bytes32 destinationChainIdentifier, IncentiveDescription calldata incentive) {
        // 1. Gas limits
        if (incentive.maxGasDelivery < minGasFor[destinationChainIdentifier]) revert NotEnoughIncentives();
        if (incentive.maxGasAck < minGasFor[bytes32(0)]) revert NotEnoughIncentives();

        // 2. Gas prices
        // You need to provide more than 10% gas than spent on this transaction.
        if (incentive.priceOfAckGas < tx.gasprice * 11 / 10) revert NotEnoughIncentives();
        _;
    }

    modifier onlyGARP() {
        if (msg.sender != address(GARP)) revert InvalidCaller();
        _;
    }

    modifier verifySourceChainAddress(bytes32 sourceChainIdentifier, bytes calldata fromApplication) {
        if (keccak256(fromApplication) != keccak256(chainIdentifierToDestinationAddress[sourceChainIdentifier])) revert InvalidSourceApplication();
        _;
    }

    function _handleError(bytes memory err) pure internal returns (bytes1) {
        bytes32 errorHash = keccak256(err);
        // We can use memory sclies to get better insight into exactly the error which occured.
        // This would also allow us to reuse events.
        // However, it looks like it will significantly increase gas costs so this works for now.
        // It looks like Solidity will improve their error catch implementation which will replace this.
        if (keccak256(abi.encodeWithSelector(ExceedsSecurityLimit.selector)) == errorHash) return 0x11;
        if (keccak256(abi.encodeWithSelector(ReturnInsufficientOnReceive.selector)) == errorHash) return 0x12;
        return 0x10; // unknown error.
    }

    /// @notice Connects this CCI with another contract on another chain.
    /// @dev To simplify the implementation, each chain can only be setup once. This reduces governance risks.
    /// @param remoteCCI The bytes65 encoded address on the destination chain.
    /// @param remoteGARP The messaging router encoded address on the destination chain.
    function connectNewChain(bytes32 chainIdentifier, bytes calldata remoteCCI, bytes calldata remoteGARP) onlyOwner checkBytes65Address(remoteCCI) external {
        // Check if the chain has already been set.
        // If it has, we don't allow setting it as another. This would impact existing pools.
        if (keccak256(chainIdentifierToDestinationAddress[chainIdentifier]) != KECCACK_OF_NOTHING) revert ChainAlreadySetup();

        // Set the remote CCI. Only the first 32 bytes are checked. For most chains, this should be enough.
        chainIdentifierToDestinationAddress[chainIdentifier] = remoteCCI;

        emit RemoteImplementationSet(chainIdentifier, remoteCCI, remoteGARP);

        // Set the remote messaging router escrow.
        GARP.setRemoteEscrowImplementation(chainIdentifier, remoteGARP);
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target vault with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy vault.
     * @param chainIdentifier The target chain identifier.
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
        bytes32 chainIdentifier,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint8 toAssetIndex,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        address fromAsset,
        IncentiveDescription calldata incentive,
        bytes calldata calldata_
    ) checkBytes65Address(toVault) checkBytes65Address(toAccount) checkIncentives(chainIdentifier, incentive) external payable {
        // We need to ensure that all information is in the correct places. This ensures that calls to this contract
        // will always be decoded semi-correctly even if the input is very incorrect. This also checks that the user 
        // inputs into the swap contracts are correct while making the cross-chain interface flexible for future implementations.
        // These checks are done by modifiers.

        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystPayload.sol for the payload definition
        bytes memory data = abi.encodePacked(
            CTX0_ASSET_SWAP,
            abi.encodePacked(
                uint8(20),      // EVM addresses are 20 bytes.
                bytes32(0),     // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(msg.sender)  // Use abi.encode to encode address into 32 bytes
            ),
            toVault,    // Length is expected to be pre-encoded.
            toAccount,  // Length is expected to be pre-encoded.
            U,
            toAssetIndex,
            minOut,
            fromAmount,
            abi.encodePacked(
                uint8(20),      // EVM addresses are 20 bytes.
                bytes32(0),     // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(fromAsset)  // Use abi.encode to encode address into 32 bytes
            ),
            uint32(block.number),   // This is the same as block.number mod 2**32-1
            uint16(calldata_.length),   // max length of calldata is 2**16-1 = 65535 bytes which should be more than plenty.
            calldata_
        );

        GARP.escrowMessage{value: msg.value}(
            chainIdentifier,
            chainIdentifierToDestinationAddress[chainIdentifier],
            data,
            incentive
        );
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target vault with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy vault.
     * @param chainIdentifier The target chain identifier. 
     * @param toVault The target vault on the target chain. Encoded in 64 + 1 bytes.
     * @param toAccount The recipient of the transaction on the target chain. Encoded in 64 + 1 bytes.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut An array of minout describing: [the minimum number of vault tokens, the minimum number of reference assets]
     * @param fromAmount Escrow related value. The amount returned if the swap fails.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: abi.encodePacket(bytes20(<address>), <data>).
     */
    function sendCrossChainLiquidity(
        bytes32 chainIdentifier,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint256 U,
        uint256[2] calldata minOut,
        uint256 fromAmount,
        IncentiveDescription calldata incentive,
        bytes memory calldata_
    ) checkBytes65Address(toVault) checkBytes65Address(toAccount) checkIncentives(chainIdentifier, incentive) external payable {
        // We need to ensure that all information is in the correct places. This ensures that calls to this contract
        // will always be decoded semi-correctly even if the input is very incorrect. This also checks that the user 
        // inputs into the swap contracts are correct while making the cross-chain interface flexible for future implementations.
        // These checks are done by modifiers.

        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystPayload.sol for the payload definition
        bytes memory data = abi.encodePacked(
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

        GARP.escrowMessage{value: msg.value}(
            chainIdentifier,
            chainIdentifierToDestinationAddress[chainIdentifier],
            data,
            incentive
        );
    }

    /**
     * @notice Cross-chain message success handler
     * @dev Should never revert.
     */
    function _onPacketSuccess(bytes32 destinationIdentifierbytes, bytes calldata data) internal {
        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromVault must be an EVM address.
        address fromVault = address(bytes20(data[ FROM_VAULT_START_EVM : FROM_VAULT_END ]));

        if (context == CTX0_ASSET_SWAP) {
            ICatalystV1Vault(fromVault).onSendAssetSuccess(
                destinationIdentifierbytes,                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                address(bytes20(data[ CTX0_FROM_ASSET_START_EVM : CTX0_FROM_ASSET_END ])),  // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // block number
            );
        }
        else if (context == CTX1_LIQUIDITY_SWAP) {
            ICatalystV1Vault(fromVault).onSendLiquiditySuccess(
                destinationIdentifierbytes,                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );
        }
        else {
            // A proper message should never get here. If the message got here, we are never going to be able to properly process it.
            revert InvalidContext(context);
        }
    }

    /**
     * @notice Cross-chain message failure handler
     * @dev Should never revert.
     */
    function _onPacketFailure(bytes32 destinationIdentifier, bytes calldata data) internal {
        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromVault must be an EVM address.
        address fromVault = address(bytes20(data[ FROM_VAULT_START_EVM : FROM_VAULT_END ]));

        if (context == CTX0_ASSET_SWAP) {
            ICatalystV1Vault(fromVault).onSendAssetFailure(
                destinationIdentifier,                                        // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                address(bytes20(data[ CTX0_FROM_ASSET_START_EVM : CTX0_FROM_ASSET_END ])),  // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // block number
            );
        }
        else if (context == CTX1_LIQUIDITY_SWAP) {
            ICatalystV1Vault(fromVault).onSendLiquidityFailure(
                destinationIdentifier,                                        // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );
        }
        else {
            // A proper message should never get here. If the message got here, we are never going to be able to properly process it.
            revert InvalidContext(context);
        }
    }

    /**
     * @notice The Acknowledgement package handler
     * @dev Should never revert.
     * @param destinationIdentifier Identifier for the destination chain
     * @param acknowledgement The acknowledgement bytes for the cross-chain swap.
     */
    function ackMessage(bytes32 destinationIdentifier, bytes32 messageIdentifier, bytes calldata acknowledgement) onlyGARP external {
        // If the transaction executed but some logic failed, an ack is sent back with an error acknowledgement.
        // This is known as "fail on ack". The package should be failed.
        bytes1 swapStatus = acknowledgement[0];
        if (swapStatus != 0x00) {
            emit SwapFailed(swapStatus);
            return _onPacketFailure(destinationIdentifier, acknowledgement);
        }
        // Otherwise, it must be a success:
        _onPacketSuccess(destinationIdentifier, acknowledgement[1:]);
    }

    /**
     * @notice The receive packet handler
     * @param sourceIdentifier Source chain identifier.
     * @param fromApplication The bytes65 encoded fromApplication.
     * @param message The message sent by the source chain.
     * @return acknowledgement The acknowledgement status of the transaction after execution.
     */
    function receiveMessage(bytes32 sourceIdentifier, bytes32 messageIdentifier, bytes calldata fromApplication, bytes calldata message) onlyGARP verifySourceChainAddress(sourceIdentifier, fromApplication) external override returns (bytes memory acknowledgement) {
        bytes1 swapStatus = _receiveMessage(sourceIdentifier, message);

        return bytes.concat(
            swapStatus,
            message
        );
    }

    /**
     * @notice Message handler
     * @param data The IBC packet
     * @return acknowledgement The status of the transaction after execution
     */
    function _receiveMessage(bytes32 sourceIdentifierbytes, bytes calldata data) internal virtual returns (bytes1 acknowledgement) {
        bytes1 context = data[CONTEXT_POS];

        // Check that toAccount is the correct length and only contains 0 bytes beyond the address.
        if (uint8(data[TO_ACCOUNT_LENGTH_POS]) != 20) revert InvalidAddress();  // Check correct length
        if (uint256(bytes32(data[TO_ACCOUNT_START:TO_ACCOUNT_START+32])) != 0) revert InvalidAddress();  // Check first 32 bytes are 0.
        if (uint96(bytes12(data[TO_ACCOUNT_START+32:TO_ACCOUNT_START_EVM])) != 0) revert InvalidAddress();  // Check the next 32-20=12 bytes are 0.
        // To vault will not be checked. If it is assumed that any error is random, then an incorrect toVault will result in the call failling.

        if (context == CTX0_ASSET_SWAP) {
            return acknowledgement = _handleOrdinarySwap(sourceIdentifierbytes, data);
        }
        if (context == CTX1_LIQUIDITY_SWAP) {
            return acknowledgement = _handleLiquiditySwap(sourceIdentifierbytes, data);
        }
        /* revert InvalidContext(context); */
        // No return here. Instead, another implementation can override this implementation. It should just keep adding ifs with returns inside:
        // acknowledgement = super._receiveMessage(...)
        // if (acknowledgement == 0x01) { if (context == CTXX) ...}
        return acknowledgement = 0x01;
    }


    function _handleOrdinarySwap(bytes32 sourceIdentifierbytes, bytes calldata data) internal returns (bytes1 status) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));
        // CCI sets dataLength > 0 if calldata is passed.
        if (dataLength != 0) {
            try ICatalystV1Vault(toVault).receiveAsset(
                sourceIdentifierbytes,                                                      // connectionId
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
            ) {
                return 0x00;
            } catch (bytes memory err) {
                return _handleError(err);
            }
        } else {
            try ICatalystV1Vault(toVault).receiveAsset(
                sourceIdentifierbytes,                                                      // connectionId
                fromVault,                                                                   // fromVault
                uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                       // toAssetIndex
                address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),           // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ])),            // minOut
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                bytes(data[ CTX0_FROM_ASSET_LENGTH_POS : CTX0_FROM_ASSET_END ]),            // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // blocknumber
            ) {
                return 0x00;
            } catch (bytes memory err) {
                return _handleError(err);
            }
        }
    }

    function _handleLiquiditySwap(bytes32 sourceIdentifierbytes, bytes calldata data) internal returns (bytes1 status) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        uint16 dataLength = uint16(bytes2(data[CTX1_DATA_LENGTH_START:CTX1_DATA_LENGTH_END]));
        // CCI sets dataLength > 0 if calldata is passed.
        if (dataLength != 0) {
            try ICatalystV1Vault(toVault).receiveLiquidity(
                sourceIdentifierbytes,                                                      // connectionId
                fromVault,                                                                  // fromVault
                address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),            // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_MIN_VAULT_TOKEN_START : CTX1_MIN_VAULT_TOKEN_END ])), // minOut
                uint256(bytes32(data[ CTX1_MIN_REFERENCE_START : CTX1_MIN_REFERENCE_END ])),// minOut
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ])),    // block number
                address(bytes20(data[ CTX1_DATA_START : CTX1_DATA_START+20 ])),             // dataTarget
                data[ CTX1_DATA_START+20 : CTX1_DATA_START+dataLength ]                     // dataArguments
            ) {
                return 0x00;
            } catch (bytes memory err) {
                return _handleError(err);
            }
        } else {
            try ICatalystV1Vault(toVault).receiveLiquidity(
                sourceIdentifierbytes,                                                      // connectionId
                fromVault,                                                                   // fromVault
                address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),            // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_MIN_VAULT_TOKEN_START : CTX1_MIN_VAULT_TOKEN_END ])), // minOut
                uint256(bytes32(data[ CTX1_MIN_REFERENCE_START : CTX1_MIN_REFERENCE_END ])),// minOut
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // blocknumber
            ) {
                return 0x00;
            } catch (bytes memory err) {
                return _handleError(err);
            }
        }
    }
}
