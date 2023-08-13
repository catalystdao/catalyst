//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";
import { IIncentivizedMessageEscrow } from "GeneralisedIncentives/src/interfaces/IIncentivizedMessageEscrow.sol";
import { ICatalystReceiver } from "./interfaces/IOnCatalyst.sol";
import { ICrossChainReceiver } from "GeneralisedIncentives/src/interfaces/ICrossChainReceiver.sol";
import { ICatalystV1Vault } from "./ICatalystV1Vault.sol";
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
    using SafeTransferLib for ERC20;
    
    //--- ERRORS ---//
    error InvalidCaller();  // Only the message router should be able to deliver messages.
    error InvalidContext(bytes1 context);
    error InvalidAddress();
    error InvalidSourceApplication();
    error SubcallOutOfGas();
    error NotEnoughIncentives();
    error ChainAlreadySetup();

    //-- Underwriting Errors --//
    error SwapAlreadyUnderwritten();
    error RefundToZeroAddress();
    error UnderwriteDoesNotExist(bytes32 identifier);
    error UnderwriteNotExpired(uint256 timeUnitilExpiry);
    error MaxUnderwriteDurationTooLong();
    error NoVaultConnection();

    //--- Structs ---//

    struct UnderwritingStorage {
        uint256 tokens;     // 1 slot
        address refundTo;   // 2 slot: 20/32
        uint80 expiry;      // 2 slot: 30/32
    }

    bytes32 constant KECCACK_OF_NOTHING = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470;

    //--- Events ---//

    event SwapFailed(bytes1 error);
    event RemoteImplementationSet(bytes32 chainIdentifier, bytes remoteCCI, bytes remoteGARP);
    
    event MinGasFor(
        bytes32 identifier,
        uint48 minGas
    );

    //-- Underwriting Events --//
    /**
     * @dev The collatoral and thus the reward for expiring an underwrite can be derived through fromAmount. 
     * All the arguments to derive the identifier can be found within event. This reduces the complexity for clients
     * calling expired underwrites.
     */
    event UnderwriteSwap(
        bytes32 indexed identifier,
        address indexed underwriter,
        uint80 expiry,
        bytes32 getSwapIdentifier,
        address targetVault,
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
        bytes cdata
    );

    event FulfillUnderwrite(
        bytes32 indexed identifier
    );

    event ExpireUnderwrite(
        bytes32 indexed identifier,
        address expirer,
        uint256 reward
    );

    //--- Config ---//
    IIncentivizedMessageEscrow public immutable GARP; // Set on deployment


    constructor(address GARP_) {
        require(address(GARP_) != address(0));  // dev: GARP_ cannot be zero address
        GARP = IIncentivizedMessageEscrow(GARP_);
    }


    //-- Underwriting Config--//

    uint256 constant public UNDERWRITING_UNFULFILLED_FEE = 1035;  // 3,5% extra as collatoral.
    uint256 constant public UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR = 1000;

    uint256 constant public EXPIRE_CALLER_REWARD = 350;  // 35% of the 3,5% = 1,225%. Of $1000 = $12,25
    uint256 constant public EXPIRE_CALLER_REWARD_DENOMINATOR = 1000;


    //--- Storage ---//

    // If not set, then the identifier is abi.encodePacked(uint8(20), bytes32(0), bytes32(msg.sender))
    mapping(bytes32 => bytes) public chainIdentifierToDestinationAddress;

    mapping(bytes32 => uint48) public minGasFor;

    //-- Underwriting Storage --//
    /// @notice Sets the maximum duration for underwriting.
    /// @dev Should be set long enough for all swaps to be able to confirm + a small buffer
    /// Should also be set long enough to not take up an excess amount of escrow usage.
    uint256 public maxUnderwritingDuration = 12 hours;

     mapping(bytes32 => UnderwritingStorage) public underwritingStorage;

    //-- Admin--//

    /// @notice Allow updating of the minimum gas limit.
    /// @dev Set chainIdentifier to 0 for gas for ack. 
    function setMinGasFor(bytes32 chainIdentifier, uint48 minGas) external onlyOwner {
        minGasFor[chainIdentifier] = minGas;

        emit MinGasFor(chainIdentifier, minGas);
    }

    function setMaxUnderwritingDuration(uint256 newMaxUnderwriteDuration) onlyOwner external {
        // If the underwriting duration is too long, users can freeze up a lot of value for not a lot of cost.
        if (newMaxUnderwriteDuration > 15 days) revert MaxUnderwriteDurationTooLong();

        maxUnderwritingDuration = newMaxUnderwriteDuration;
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

    //-- Functions --//

    function _handleError(bytes memory err) pure internal returns (bytes1) {
        bytes32 errorHash = keccak256(err);
        // We can use memory sclies to get better insight into exactly the error which occured.
        // This would also allow us to reuse events.
        // However, it looks like it will significantly increase gas costs so this works for now.
        // It looks like Solidity will improve their error catch implementation which will replace this.
        if (keccak256(abi.encodeWithSelector(ExceedsSecurityLimit.selector)) == errorHash) return 0x11;
        if (keccak256(abi.encodeWithSelector(ReturnInsufficientOnReceive.selector)) == errorHash) return 0x12;
        if (keccak256(abi.encodeWithSelector(VaultNotConnected.selector)) == errorHash) return 0x13;
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
    function _receiveMessage(bytes32 sourceIdentifier, bytes calldata data) internal virtual returns (bytes1 acknowledgement) {
        bytes1 context = data[CONTEXT_POS];

        // Check that toAccount is the correct length and only contains 0 bytes beyond the address.
        if (uint8(data[TO_ACCOUNT_LENGTH_POS]) != 20) revert InvalidAddress();  // Check correct length
        if (uint256(bytes32(data[TO_ACCOUNT_START:TO_ACCOUNT_START+32])) != 0) revert InvalidAddress();  // Check first 32 bytes are 0.
        if (uint96(bytes12(data[TO_ACCOUNT_START+32:TO_ACCOUNT_START_EVM])) != 0) revert InvalidAddress();  // Check the next 32-20=12 bytes are 0.
        // To vault will not be checked. If it is assumed that any error is random, then an incorrect toVault will result in the call failling.

        if (context == CTX0_ASSET_SWAP) {
            return acknowledgement = _handleOrdinarySwap(sourceIdentifier, data);
        }
        if (context == CTX1_LIQUIDITY_SWAP) {
            return acknowledgement = _handleLiquiditySwap(sourceIdentifier, data);
        }
        if (context == CTX2_ASSET_SWAP_PLEASE_UNDERWRITE) {
            return acknowledgement = _pleaseFill(sourceIdentifier, data);
        }
        if (context == CTX3_ASSET_SWAP_PURPOSE_UNDERWRITE) {
            // return acknowledgement = _purposeFill(sourceIdentifier, data);
        }
        /* revert InvalidContext(context); */
        // No return here. Instead, another implementation can override this implementation. It should just keep adding ifs with returns inside:
        // acknowledgement = super._receiveMessage(...)
        // if (acknowledgement == 0x01) { if (context == CTXX) ...}
        return acknowledgement = 0x01;
    }


    function _handleOrdinarySwap(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 status) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));
        // CCI sets dataLength > 0 if calldata is passed.
        if (dataLength != 0) {
            try ICatalystV1Vault(toVault).receiveAsset(
                sourceIdentifier,                                                      // connectionId
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
                sourceIdentifier,                                                      // connectionId
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

    function _handleLiquiditySwap(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 status) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        uint16 dataLength = uint16(bytes2(data[CTX1_DATA_LENGTH_START:CTX1_DATA_LENGTH_END]));
        // CCI sets dataLength > 0 if calldata is passed.
        if (dataLength != 0) {
            try ICatalystV1Vault(toVault).receiveLiquidity(
                sourceIdentifier,                                                      // connectionId
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
                sourceIdentifier,                                                      // connectionId
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

    //--- Underwriting ---//

    function getSwapIdentifier(
        address targetVault,
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
        uint16 underwritePercentageX16,
        bytes calldata cdata
    ) public pure returns (bytes32 identifier) {
        return identifier = keccak256(
            abi.encodePacked(
                targetVault,
                toAsset,
                U,
                minOut,
                toAccount,
                fromAmount,
                underwritePercentageX16,
                cdata
            )
        );
    }


    /**
     * @notice Underwrites a swap and check if there is a connection. 
     * @dev sourceIdentifier and fromVault are not verified that they belong to the message.
     * As a result, care should be placed on correctly reading these.
     */
    function underwriteAndCheckConnection(
        bytes32 sourceIdentifier,
        bytes calldata fromVault, // -- Conection Check
        address refundTo,     // -- Underwriting State
        address targetVault,  // -- Swap information
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
        uint16 underwritePercentageX16,
        bytes calldata cdata
    ) external {
        if (!ICatalystV1Vault(targetVault)._vaultConnection(sourceIdentifier, fromVault)) revert NoVaultConnection();

        underwrite(
            refundTo,
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
            underwritePercentageX16,
            cdata
        );
    }

    /**
     * @notice Underwrite a swap.
     * There are a few important properties that underwriters should be aware of:
     * 1. There is nothing which checks if the connection is valid on this call. This check should be done seperately. If this is important, use the underwriteAndCheck function.
     */
    function underwrite(
        address refundTo,     // -- Underwriting State
        address targetVault,  // -- Swap information
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
        uint16 underwritePercentageX16,
        bytes calldata cdata
    ) public {
        // Do not allow refundTo zero address. This field will be used to check if a swap has already been
        // underwritten, as a result disallow zero address. (Is also makes no sense from an underwriting perspective)  
        if (refundTo == address(0)) revert RefundToZeroAddress();

        // Get the swap identifier.
        // When an incoming swap says: "_pleaseUnderwrite (CTX2)", the same identifier will be computed based on the provided
        // CTX0 (asset swap) arguments. Assuming that the swap has already been executed, these arguments are constant and known.
        // As a result, once the swap arrives (and the underwriter is competent), the same identifier will be computed and matched.
        // For "_purposeFill (CTX3)", how the identifier is computed doesn't matter. Only that it is relativly unigue. To simplify
        // the implementations, both methods use the same identifier deriviation.
        // For other implementations: This arguments the identifier is based on should be the same but the hashing
        // algorithm doesn't matter. It is only used and computed on the destination chain.
        bytes32 identifier = getSwapIdentifier(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
            underwritePercentageX16,
            cdata
        );
        // Observation: A swapper can execute multiple swaps which return the same U. 
        // In these cases, it would not be possible to underwrite the second swap until after the
        // first swap has arrived. This can be counteracted by either:
        // 1. Changing U. U is pretty *volatile*, to this would have to be deliberate. Note that the 
        // identifier also contains fromAmount and as a result, it is incredibly unlikely that someone
        // would be able to generate identical identifiers. (especially if fee > 0.)
        // 2. Add random noise to minOut. For tokens with 18 decimals, this noise can be as small as 1e-12
        // but the chance that 2 swaps collide would be 1 in a million'th. (literally 1/(1e(18-12)))
        // 3. Add either a counter to cdata or some noise.

        // For most implementations, the observation ignored entirely.

        // Ensure the swap hasn't already been underwritten by checking if refundTo is set.
        if (underwritingStorage[identifier].refundTo != address(0)) revert SwapAlreadyUnderwritten();

        // Get the number of purchased units from the vault. This uses a custom call which doesn't return
        // any assets.
        // This calls also escrows the purchasedTokens on the vault.
        // Importantly! The connection is not checked here. Instead it is checked when the
        // message arrives. As a result, the underwriter should verify that a message is good.
        uint256 purchasedTokens = ICatalystV1Vault(targetVault).underwriteAsset(
            identifier,
            toAsset,
            U,
            minOut
        );

        // Collect tokens and collatoral from underwriter.
        ERC20(toAsset).safeTransferFrom(
            msg.sender, 
            address(this),
            purchasedTokens * (
                UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR+UNDERWRITING_UNFULFILLED_FEE
            )/UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR
        );

        // Send the assets to the user.
        ERC20(toAsset).safeTransfer(toAccount, purchasedTokens);

        // Figure out if the user wants to execute additional logic.
        // Note that this logic is not contained within a try catch. It could fail.
        // An underwrite should simulate the tx execution before submitting the transaction as otherwise
        // they could be out the associated gas.
        uint16 calldataLength = uint16(bytes2(cdata[0:2]));
        if (calldataLength > 0) {
            address dataTarget = address(bytes20(cdata[2:2+20]));
            bytes calldata customCalldata = cdata[2+20:2+20+calldataLength];
            ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, customCalldata);
        }


        underwritingStorage[identifier] = UnderwritingStorage({
            tokens: purchasedTokens,
            refundTo: refundTo,
            expiry: uint80(uint256(block.timestamp) + uint256(maxUnderwritingDuration))  // Should never overflow.
        });
    }

    /**
     * @notice Resolves unexpired underwrites so that the escrowed value can be freed.
     * @dev The underwrite owner can expire it at any time. Other callers needs to wait until after expiry. 
     */
    function expireUnderwrite(
        address targetVault,
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
        uint16 underwritePercentageX16,
        bytes calldata cdata
    ) external {
        bytes32 identifier = getSwapIdentifier(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
            underwritePercentageX16,
            cdata
        );

        UnderwritingStorage storage underwriteState = underwritingStorage[identifier];
        // Check that the refundTo address is set. (Indicated that the underwrite exists.)
        if (underwriteState.refundTo == address(0)) revert UnderwriteDoesNotExist(identifier);
        
        // Check that the underwriting can be expired. If the msg.sender is the refundTo address, then it can be expired at any time.
        if (msg.sender != underwriteState.refundTo) {
            // Otherwise, the expiry time must have been passed.
            if (underwriteState.expiry > block.timestamp) revert UnderwriteNotExpired(underwriteState.expiry - block.timestamp);
        }
        uint256 underWrittenTokens = underwriteState.tokens;
        // The next line acts as reentry protection. When the storage is deleted underwriteState.refundTo == address(0) will be true.
        delete underwritingStorage[identifier];

        // Delete the escrow
        ICatalystV1Vault(targetVault).deleteUnderwriteAsset(identifier, fromAmount, toAsset);

        // Get the collatoral.
        uint256 refundAmount = underWrittenTokens * (
            UNDERWRITING_UNFULFILLED_FEE
        )/UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR;
        // Send the coded shares of the collatoral to the vault the rest to the expirer.

        uint256 expireShare = refundAmount * EXPIRE_CALLER_REWARD / EXPIRE_CALLER_REWARD_DENOMINATOR;
        ERC20(toAsset).safeTransfer(msg.sender, expireShare);
        uint256 vaultShare = refundAmount - expireShare;
        ERC20(toAsset).safeTransfer(targetVault, vaultShare);

        // The underwriting storage has already been deleted.
    }

    function _matchUnderwrite(
        bytes32 identifier,
        address toAsset,
        address vault,
        uint256 underwrittenU,
        uint256 incomingU
    ) internal returns (uint256 unitExcess) {
        UnderwritingStorage storage underwriteState = underwritingStorage[identifier];
        // Load number of tokens from storage.
        uint256 underwrittenTokenAmount = underwriteState.tokens;
        address refundTo = underwriteState.refundTo;
        if (refundTo == address(0)) return unitExcess = incomingU;

        // If the swap was filled less than 100%, then fail the swap.
        // This is only possible for purpose underwrite.
        if (underwrittenU > incomingU) {
            // Set the most significant bit to signal that the swap reverted.
            return unitExcess = 2**255;
        }
        // Reentry protection. No external calls are allowed before this line.
        delete underwritingStorage[identifier];

        // Delete escrow information and send tokens to this contract.
        ICatalystV1Vault(vault).releaseUnderwriteAsset(identifier, underwrittenTokenAmount, toAsset);

        // Also refund the collatoral.
        uint256 refundAmount = underwrittenTokenAmount * (
            UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR+UNDERWRITING_UNFULFILLED_FEE
        )/UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR;
        ERC20(toAsset).safeTransfer(refundTo, refundAmount);

        // uint256 unitExcess is initialized to 0.
        if (underwrittenU < incomingU) {
            unitExcess = incomingU - underwrittenU;
        }
    }
    
    function sendCrossChainPleaseUnderwrite(
        bytes32 chainIdentifier,
        bytes calldata toVault,
        bytes calldata toAccount,
        uint8 toAssetIndex,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        address fromAsset,
        uint16 underwritePercentageX16,
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
            CTX2_ASSET_SWAP_PLEASE_UNDERWRITE,
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
            uint32(block.number),   // This is the same as block.number mod 2**32-1,
            uint16(underwritePercentageX16),
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

    function _pleaseFill(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 acknowledgement) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        // Check that there is a connection. Otherwise, send a bad (non 0x00) ack back.
        if (!ICatalystV1Vault(toVault)._vaultConnection(sourceIdentifier, fromVault)) return acknowledgement = 0x23;

        // uint16 dataLength = uint16(bytes2(data[CTX2_DATA_LENGTH_START:CTX2_DATA_LENGTH_END]));

        // Select excess calldata. Excess calldata is not decoded.
        bytes calldata cdata = data[CTX2_DATA_LENGTH_START:];

        // Get the toAsset
        uint8 toAssetIndex = uint8(data[CTX2_TO_ASSET_INDEX_POS]);
        address toAsset = ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex);

        // Get the rest of the swap parameters.
        address toAccount = address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]));
        uint256 U = uint256(bytes32(data[ UNITS_START : UNITS_END ]));
        uint256 minOut = uint256(bytes32(data[ CTX2_MIN_OUT_START : CTX2_MIN_OUT_END ]));
        uint256 fromAmount = uint256(bytes32(data[ CTX2_FROM_AMOUNT_START : CTX2_FROM_AMOUNT_END ]));
        uint16 underwritePercentageX16 = uint16(bytes2(data[CTX2_UW_INCENTIVE_START:CTX2_UW_INCENTIVE_END]));

        // Get the underwriting identifier.
        bytes32 identifier = getSwapIdentifier(
            toVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
            underwritePercentageX16,
            cdata
        );

        // Check if the swap has been underwritten. If it has, return funds to underwriter.
        uint256 unusedUnits = _matchUnderwrite(
            identifier,
            toAsset,
            toVault,
            U,
            U
        );

        if (unusedUnits == 2**255) {
            return acknowledgement = 0x21; // Not enough units were provided..
        }

        if (U == unusedUnits) {
            // The swap hasn't been underwritten. Lets execute the swap properly:
            return acknowledgement = _handleOrdinarySwap(sourceIdentifier, data);
        } else {
            // todo: There shouldn't ever be a case where this is the case.
        }

        return acknowledgement = 0x00;
    }

/* 

    function sendCrossChainPurposeUnderwrite(
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
            CTX2_ASSET_SWAP_PLEASE_UNDERWRITE,
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

        GARP.escrowMessage{value: msg.value}(
            chainIdentifier,
           chainIdentifierToDestinationAddress[chainIdentifier],
            data,
            incentive
        );
    }

    This function is used to pre-specify the identifier to backfill underwritten swaps.
    While _pleaseFill could technically be used for it, it can't because the identifier contains U. 
    function _purposeFill(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 acknowledgement) {
        // 1. Generate the underwriting ID.

        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        // Check that there is a connection.
        if (!ICatalystV1Vault(toVault)._vaultConnection(sourceIdentifier, fromVault)) return 0x23;

        // uint16 dataLength = uint16(bytes2(data[CTX2_DATA_LENGTH_START:CTX2_DATA_LENGTH_END]));

        // Select excess calldata. Excess calldata is not decoded.
        bytes calldata cdata = data[CTX2_DATA_LENGTH_START:];

        // Get the toAsset
        uint8 toAssetIndex = uint8(data[CTX2_TO_ASSET_INDEX_POS]);
        address toAsset = ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex);

        // Get the rest of the swap parameters.
        address toAccount = address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]));
        uint256 U = uint256(bytes32(data[ UNITS_START : UNITS_END ]));
        uint256 minOut = uint256(bytes32(data[ CTX2_MIN_OUT_START : CTX2_MIN_OUT_END ]));
        uint256 fromAmount = uint256(bytes32(data[ CTX2_FROM_AMOUNT_START : CTX2_FROM_AMOUNT_END ]));
        uint16 underwritePercentageX16 = uint16(bytes2(data[ CTX2_UW_INCENTIVE_START : CTX2_UW_INCENTIVE_END ]));

        // Get the underwriting identifier.
        bytes32 identifier = getSwapIdentifier(
            toVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
            underwritePercentageX16,
            cdata
        );

        // 2. Check if the swap has been underwritten. If it has, return funds to underwriter.
        uint256 unusedUnits = _matchUnderwrite(
            identifier,
            toAsset,
            toVault,
            U,
            U
        );

        if (unusedUnits == 2**255) {
            return 0x21; // Not enough units were provided..
        }

        if (U == unusedUnits) {
            // The swap hasn't been underwritten. Lets execute the swap properly:
            return acknowledgement = _handleOrdinarySwap(sourceIdentifier, data);
        }
        if (unusedUnits > 0) {
            // Use unspent units to purchase tokens. This is fallback logic. The payload won't be executed again.
            address fallbackAccount = address(bytes20(data[ CTX2_TO_ACCOUNT_FALLBACK_START_EVM : CTX2_TO_ACCOUNT_FALLBACK_END ]));
            try ICatalystV1Vault(toVault).receiveAsset(
                sourceIdentifier,                                                           // connectionId
                fromVault,                                                                  // fromVault
                toAssetIndex,                                                               // toAssetIndex
                fallbackAccount,                                                            // toAccount
                uint256(unusedUnits),                                                       // units
                uint256(0),                                                                 // minOut
                fromAmount,                                                                 // fromAmount
                bytes(data[ CTX2_FROM_ASSET_LENGTH_POS : CTX2_FROM_ASSET_END ]),            // fromAsset
                uint32(bytes4(data[ CTX2_BLOCK_NUMBER_START : CTX2_BLOCK_NUMBER_END ]))     // block number
            ) {
                // Return outside try catch to save a bit of deployment gas.
            } catch (bytes memory err) {
                // Since the swap already properly executed, we shouldn't have to do anything extra.
                // Instead, the best fallback is to burn the unspent units.
            }
            return acknowledgement = 0x00;
        }

        return acknowledgement = 0x00;
    }
*/
}
