//SPDX-License-Identifier: MIT

pragma solidity ^0.8.19;

import {ERC20} from 'solmate/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/utils/SafeTransferLib.sol';
import { IMessageEscrowStructs } from "GeneralisedIncentives/src/interfaces/IMessageEscrowStructs.sol";
import { IIncentivizedMessageEscrow } from "GeneralisedIncentives/src/interfaces/IIncentivizedMessageEscrow.sol";
import { ICatalystReceiver } from "./interfaces/IOnCatalyst.sol";
import { ICatalystV1Vault } from "./ICatalystV1Vault.sol";
import { Ownable} from "openzeppelin-contracts/contracts/access/Ownable.sol";
import "./ICatalystV1Vault.sol";
import "./interfaces/ICatalystV1VaultState.sol"; // structs
import "./CatalystPayload.sol";
import { Bytes65 } from "GeneralisedIncentives/src/utils/Bytes65.sol";

import { ICatalystChainInterface } from "./interfaces/ICatalystChainInterface.sol";

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
contract CatalystChainInterface is ICatalystChainInterface, Ownable, Bytes65 {
    using SafeTransferLib for ERC20;
    
    //--- ERRORS ---//
     // Only the message router should be able to deliver messages.
    error InvalidCaller(); // 48f5c3ed 
    error InvalidContext(bytes1 context); // 9f769791
    error InvalidAddress(); // e6c4247b
    error InvalidSourceApplication(); // 003923e0
    error SubcallOutOfGas(); // d271060e
    error NotEnoughIncentives(uint256 expected, uint256 actual); // 6de78246
    error ChainAlreadySetup(); // b8e35614
    
    //-- Underwriting Errors --//
    error SwapAlreadyUnderwritten(); // d0c27c9f
    error UnderwriteDoesNotExist(bytes32 identifier); // ae029d69
    error UnderwriteNotExpired(uint256 blocksUnitilExpiry); // 62141db5
    error MaxUnderwriteDurationTooLong(); // 3f6368aa
    error MaxUnderwriteDurationTooShort(); // 6229dcd0
    error NoVaultConnection(); // ea66ca6d
    error MaliciousVault(); // 847ca49a
    error SwapRecentlyUnderwritten(); // 695b3a94

    //--- Events ---//

    event SwapFailed(bytes1 error);
    event RemoteImplementationSet(bytes32 chainIdentifier, bytes remoteCCI, bytes remoteGARP);
    event MaxUnderwriteDuration(uint256 newMaxUnderwriteDuration);
    
    event MinGasFor(
        bytes32 identifier,
        uint48 minGas
    );

    //-- Underwriting Events --//

    event SwapUnderwritten(
        bytes32 indexed identifier,
        address indexed underwriter,
        uint96 expiry,
        address targetVault,
        address toAsset,
        uint256 U,
        address toAccount,
        uint256 outAmount
    );

    event FulfillUnderwrite(
        bytes32 indexed identifier
    );

    event ExpireUnderwrite(
        bytes32 indexed identifier,
        address expirer,
        uint256 reward
    );

    //--- Structs ---//

    struct UnderwritingStorage {
        uint256 tokens;     // 1 slot
        address refundTo;   // 2 slot: 20/32
        uint96 expiry;      // 2 slot: 32/32
    }

    uint256 constant INITIAL_MAX_UNDERWRITE_DURATION = 24 hours / 6 seconds; // Is 8 hours if the block time is 2 seconds, 48 hours if the block time is 12 seconds. Not a great initial value but better than nothing.
    uint256 constant MAX_UNDERWRITE_DURATION = 14 days / 3 seconds;  // Is 9.3 days if the block time is 2 seconds, 56 days if the block time is 12 seconds. Not a great measure but better than no protection.
    uint256 constant MIN_UNDERWRITE_DURATION = 1 hours / 12 seconds;  // Is 10 minutes if the block time is 2 seconds, 1 hour if the block time is 12 seconds. Not a great measure but better than no protection.

    //--- Config ---//
    IIncentivizedMessageEscrow public immutable GARP; // Set on deployment


    //-- Underwriting Config--//

    // How many blocks should there be between when an identifier can be underwritten.
    uint24 constant BUFFER_BLOCKS = 4;

    uint256 constant public UNDERWRITING_COLLATORAL = 35;  // 3,5% extra as collatoral.
    uint256 constant public UNDERWRITING_COLLATORAL_DENOMINATOR = 1000;

    uint256 constant public EXPIRE_CALLER_REWARD = 350;  // 35% of the 3,5% = 1,225%. Of $1000 = $12,25
    uint256 constant public EXPIRE_CALLER_REWARD_DENOMINATOR = 1000;


    //--- Storage ---//

    /// @notice The destination address on the chain by chain identifier.
    mapping(bytes32 => bytes) public chainIdentifierToDestinationAddress;

    /// @notice The minimum amount of gas for a specific chain. bytes32(0) indicates ack.
    mapping(bytes32 => uint48) public minGasFor;

    //-- Underwriting Storage --//
    /// @notice Sets the maximum duration for underwriting.
    /// @dev Should be set long enough for all swaps to be able to confirm + a small buffer
    /// Should also be set long enough to not take up an excess amount of escrow usage.
    uint256 public maxUnderwritingDuration = INITIAL_MAX_UNDERWRITE_DURATION;

    /// @notice Maps underwriting identifiers to underwriting state.
    /// refundTo can be checked to see if the ID has been underwritten.
     mapping(bytes32 => UnderwritingStorage) public underwritingStorage;


    constructor(address GARP_, address defaultOwner) {
        require(address(GARP_) != address(0));  // dev: GARP_ cannot be zero address
        GARP = IIncentivizedMessageEscrow(GARP_);
        _transferOwnership(defaultOwner);

        emit MaxUnderwriteDuration(INITIAL_MAX_UNDERWRITE_DURATION);
    }


    //-- Admin--//

    /// @notice Allow updating of the minimum gas limit.
    /// @dev Set chainIdentifier to 0 for gas for ack. 
    function setMinGasFor(bytes32 chainIdentifier, uint48 minGas) override external onlyOwner {
        minGasFor[chainIdentifier] = minGas;

        emit MinGasFor(chainIdentifier, minGas);
    }

    /// @notice Sets the new max underwrite duration, which is the period of time
    /// before an underwrite can be expired. When an underwrite is expired, the underwriter
    /// loses all capital provided.
    /// @dev This function can be exploited by the owner. By setting newMaxUnderwriteDuration to (almost) 0 right before someone calls underwrite and then
    /// expiring them before the actual swap arrives. The min protection here is not sufficient since it needs to be well into 
    /// when a message can be validated. As a result, the owner of this contract should be a timelock which underwriters monitor.
    function setMaxUnderwritingDuration(uint256 newMaxUnderwriteDuration) onlyOwner override external {
        if (newMaxUnderwriteDuration <= MIN_UNDERWRITE_DURATION) revert MaxUnderwriteDurationTooShort();
        // If the underwriting duration is too long, users can freeze up a lot of value for not a lot of cost.
        if (newMaxUnderwriteDuration > MAX_UNDERWRITE_DURATION) revert MaxUnderwriteDurationTooLong();

        maxUnderwritingDuration = newMaxUnderwriteDuration;
        
        emit MaxUnderwriteDuration(newMaxUnderwriteDuration);
    }

    modifier checkRouteDescription(ICatalystV1Vault.RouteDescription calldata routeDescription) {
        // -- Check incentives -- //

        ICatalystV1Vault.IncentiveDescription calldata incentive = routeDescription.incentive;
        // 1. Gas limits
        if (incentive.maxGasDelivery < minGasFor[routeDescription.chainIdentifier]) revert NotEnoughIncentives(minGasFor[routeDescription.chainIdentifier], incentive.maxGasDelivery);
        if (incentive.maxGasAck < minGasFor[bytes32(0)]) revert NotEnoughIncentives(minGasFor[bytes32(0)], incentive.maxGasAck);

        // 2. Gas prices
        // The gas price of ack has to be 10% higher than the gas price spent on this transaction.
        if (incentive.priceOfAckGas < tx.gasprice * 11 / 10) revert NotEnoughIncentives(tx.gasprice * 11 / 10, incentive.priceOfAckGas);

        // -- Check Address Lengths -- //

        // toAccount
        if (!_checkBytes65(routeDescription.toAccount)) revert InvalidBytes65Address();

        // toVault
        if (!_checkBytes65(routeDescription.toVault)) revert InvalidBytes65Address();

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

    //-- Transparent viewer --//

    /// @notice Estimate the addition verification cost beyond the 
    /// cost paid to the relayer.
    function estimateAdditionalCost() override external view returns(address asset, uint256 amount) {
        (asset, amount) = GARP.estimateAdditionalCost();
    }

    //-- Functions --//

    /// @notice matches the hash of error calldata to common revert functions
    /// and then reverts a relevant ack which can be exposed on the origin to provide information
    /// about why the transaction didn't execute as expected.
    function _handleError(bytes memory err) pure internal returns (bytes1) {
        // To only get the error identifier, only use the first 8 bytes. This lets us add additional error
        // data for easier debugger on trace.
        bytes8 errorIdentifier = bytes8(err);
        // We can use memory sclies to get better insight into exactly the error which occured.
        // This would also allow us to reuse events.
        // However, it looks like it will significantly increase gas costs so this works for now.
        // It looks like Solidity will improve their error catch implementation which will replace this.
        if (bytes8(abi.encodeWithSelector(ExceedsSecurityLimit.selector)) == errorIdentifier) return 0x11;
        if (bytes8(abi.encodeWithSelector(ReturnInsufficient.selector)) == errorIdentifier) return 0x12;
        if (bytes8(abi.encodeWithSelector(VaultNotConnected.selector)) == errorIdentifier) return 0x13;
        return 0x10; // unknown error.
    }

    /// @notice Connects this CCI with another contract on another chain.
    /// @dev To simplify the implementation, each chain can only be setup once. This reduces governance risks.
    /// @param remoteCCI The bytes65 encoded address on the destination chain.
    /// @param remoteGARP The messaging router encoded address on the destination chain.
    function connectNewChain(bytes32 chainIdentifier, bytes calldata remoteCCI, bytes calldata remoteGARP) onlyOwner checkBytes65Address(remoteCCI) override external {
        // Check if the chain has already been set.
        // If it has, we don't allow setting it as another. This would impact existing pools.
        if (chainIdentifierToDestinationAddress[chainIdentifier].length != 0) revert ChainAlreadySetup();

        // Set the remote CCI.
        chainIdentifierToDestinationAddress[chainIdentifier] = remoteCCI;

        emit RemoteImplementationSet(chainIdentifier, remoteCCI, remoteGARP);

        // Set the remote messaging router escrow.
        GARP.setRemoteImplementation(chainIdentifier, remoteGARP);
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target vault with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy vault.
     * @param routeDescription A cross-chain route description which contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param toAssetIndex The index of the asset the user wants to buy in the target vault.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut The minimum number output of tokens on the target chain.
     * @param fromAmount Escrow related value. The amount returned if the swap fails.
     * @param fromAsset Escrow related value. The asset that was sold.
     * @param underwriteIncentiveX16 The payment for underwriting the swap (out of type(uint16).max)
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: abi.encodePacket(bytes20(<address>), <data>).
     */
    function sendCrossChainAsset(
        ICatalystV1Vault.RouteDescription calldata routeDescription,
        uint8 toAssetIndex,
        uint256 U,
        uint256 minOut,
        uint256 fromAmount,
        address fromAsset,
        uint16 underwriteIncentiveX16,
        bytes calldata calldata_
    ) checkRouteDescription(routeDescription) override external payable {
        // We need to ensure that all information is in the correct places. This ensures that calls to this contract
        // will always be decoded semi-correctly even if the input is very incorrect. This also checks that the user 
        // inputs into the swap contracts are correct while making the cross-chain interface flexible for future implementations.
        // These checks are done by the modifier.

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
            abi.encodePacked(
                routeDescription.toVault,    // Length is expected to be pre-encoded.
                routeDescription.toAccount,  // Length is expected to be pre-encoded.
                U,
                toAssetIndex,
                minOut,
                fromAmount
            ),
            abi.encodePacked(
                uint8(20),      // EVM addresses are 20 bytes.
                bytes32(0),     // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(fromAsset)  // Use abi.encode to encode address into 32 bytes
            ),
            uint32(block.number),   // This is the same as block.number mod 2**32-1
            uint16(underwriteIncentiveX16),
            uint16(calldata_.length),   // max length of calldata is 2**16-1 = 65535 bytes which should be more than plenty.
            calldata_
        );

        GARP.submitMessage{value: msg.value}(
            routeDescription.chainIdentifier,
            chainIdentifierToDestinationAddress[routeDescription.chainIdentifier],
            data,
            routeDescription.incentive
        );
    }

    /**
     * @notice Packs cross-chain swap information into a bytearray and sends it to the target vault with IBC.
     * @dev Callable by anyone but this cannot be abused since the connection management ensures no
     * wrong messages enter a healthy vault.
     * @param routeDescription A cross-chain route description which contains the chainIdentifier, toAccount, toVault and relaying incentive.
     * @param U The calculated liquidity reference. (Units)
     * @param minOut An array of minout describing: [the minimum number of vault tokens, the minimum number of reference assets]
     * @param fromAmount Escrow related value. The amount returned if the swap fails.
     * @param calldata_ Data field if a call should be made on the target chain.
     * Encoding depends on the target chain, with EVM: abi.encodePacket(bytes20(<address>), <data>).
     */
    function sendCrossChainLiquidity(
        ICatalystV1Vault.RouteDescription calldata routeDescription,
        uint256 U,
        uint256[2] calldata minOut,
        uint256 fromAmount,
        bytes memory calldata_
    ) checkRouteDescription(routeDescription) override external payable {
        // We need to ensure that all information is in the correct places. This ensures that calls to this contract
        // will always be decoded semi-correctly even if the input is very incorrect. This also checks that the user 
        // inputs into the swap contracts are correct while making the cross-chain interface flexible for future implementations.
        // These checks are done by the modifier.

        // Anyone can call this function, but unless someone can also manage to pass the security check on onRecvPacket
        // they cannot drain any value. As such, the very worst they can do is waste gas.

        // Encode payload. See CatalystPayload.sol for the payload definition
        bytes memory data = abi.encodePacked(
            CTX1_LIQUIDITY_SWAP,
            abi.encodePacked(
                uint8(20),  // EVM addresses are 20 bytes.
                bytes32(0),  // EVM only uses 20 bytes. abi.encode packs the 20 bytes into 32 then we need to add 32 more
                abi.encode(msg.sender)  // Use abi.encode to encode address into 32 bytes
            ),
            routeDescription.toVault,  // Length is expected to be pre-encoded.
            routeDescription.toAccount,  // Length is expected to be pre-encoded.
            U,
            minOut[0],
            minOut[1],
            fromAmount,
            uint32(block.number),
            uint16(calldata_.length),
            calldata_
        );

        GARP.submitMessage{value: msg.value}(
            routeDescription.chainIdentifier,
            chainIdentifierToDestinationAddress[routeDescription.chainIdentifier],
            data,
            routeDescription.incentive
        );
    }

    /**
     * @notice Cross-chain message success handler
     * @dev Should never revert. (on valid messages)
     */
    function _onPacketSuccess(bytes32 destinationIdentifier, bytes calldata data) internal {
        bytes1 context = data[CONTEXT_POS];
        // Since this is a callback, fromVault must be an EVM address.
        address fromVault = address(bytes20(data[ FROM_VAULT_START_EVM : FROM_VAULT_END ]));

        if (context == CTX0_ASSET_SWAP) {
            ICatalystV1Vault(fromVault).onSendAssetSuccess(
                destinationIdentifier,                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                address(bytes20(data[ CTX0_FROM_ASSET_START_EVM : CTX0_FROM_ASSET_END ])),  // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))     // block number
            );
            return;
        }
        if (context == CTX1_LIQUIDITY_SWAP) {
            ICatalystV1Vault(fromVault).onSendLiquiditySuccess(
                destinationIdentifier,                                              // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );
            return;
        }
        // A proper message should never get here. If the message got here, we are never going to be able to properly process it.
        revert InvalidContext(context);
    }

    /**
     * @notice Cross-chain message failure handler
     * @dev Should never revert. (on valid messages)
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
            return;
        }
        if (context == CTX1_LIQUIDITY_SWAP) {
            ICatalystV1Vault(fromVault).onSendLiquidityFailure(
                destinationIdentifier,                                        // connectionId
                data[ TO_ACCOUNT_LENGTH_POS : TO_ACCOUNT_END ],                             // toAccount
                uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
                uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
                uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // block number
            );
            return;
        }
        // A proper message should never get here. If the message got here, we are never going to be able to properly process it.
        revert InvalidContext(context);
    }

    /**
     * @notice The Acknowledgement package handler
     * @dev Should never revert. (on valid messages)
     * @param destinationIdentifier Identifier for the destination chain
     * @param acknowledgement The acknowledgement bytes for the cross-chain swap.
     */
    function receiveAck(bytes32 destinationIdentifier, bytes32 messageIdentifier, bytes calldata acknowledgement) onlyGARP override external {
        // If the transaction executed but some logic failed, an ack is sent back with an error acknowledgement.
        // This is known as "fail on ack". The package should be failed.
        // The acknowledgement is prepended the message, so we need to fetch it.
        // Then, we need to ignore it when passing the data to the handlers.
        bytes1 swapStatus = acknowledgement[0];
        if (swapStatus != 0x00) {
            emit SwapFailed(swapStatus);  // The acknowledgement can be mapped to get some information about what happened.
            return _onPacketFailure(destinationIdentifier, acknowledgement[1:]);
        }
        // Otherwise, swapStatus == 0x00 which implies success.
        _onPacketSuccess(destinationIdentifier, acknowledgement[1:]);
    }

    /**
     * @notice The receive packet handler
     * @param sourceIdentifier Source chain identifier.
     * @param fromApplication The bytes65 encoded fromApplication.
     * @param message The message sent by the source chain.
     * @return acknowledgement The acknowledgement status of the transaction after execution.
     */
    function receiveMessage(bytes32 sourceIdentifier, bytes32 /* messageIdentifier */, bytes calldata fromApplication, bytes calldata message) onlyGARP verifySourceChainAddress(sourceIdentifier, fromApplication) override external returns (bytes memory acknowledgement) {
        bytes1 swapStatus = _receiveMessage(sourceIdentifier, message);

        return acknowledgement = bytes.concat(
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
        // The reason toAccount is checked is that any to account will be treated as valid. So any random error will result
        // in lost funds.

        if (context == CTX0_ASSET_SWAP) {
            return acknowledgement = _handleReceiveAsset(sourceIdentifier, data);
        }
        if (context == CTX1_LIQUIDITY_SWAP) {
            return acknowledgement = _handleReceiveLiquidity(sourceIdentifier, data);
        }
        /* revert InvalidContext(context); */
        // No return here. Instead, another implementation can override this implementation. It should just keep adding ifs with returns inside:
        // acknowledgement = super._receiveMessage(...)
        // if (acknowledgement == 0x01) { if (context == CTXX) ...}
        return acknowledgement = 0x01;
    }


    function _handleReceiveAssetFallback(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 status) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

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
        ) returns(uint256 purchasedTokens) {
            uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));
            if (dataLength != 0) {
                address dataTarget = address(bytes20(data[ CTX0_DATA_START : CTX0_DATA_START+20 ]));
                bytes calldata dataArguments = data[ CTX0_DATA_START+20 : CTX0_DATA_START+dataLength ];
                
                // Let users define custom logic which should be executed after the swap.
                // The logic is not contained within a try - except so if the logic reverts
                // the transaction will timeout and the user gets the input tokens on the sending chain.
                // If this is not desired, wrap further logic in a try - except at dataTarget.
                ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, dataArguments);
                // If dataTarget doesn't implement onCatalystCall BUT implements a fallback function, the call will still succeed.
            }
            return 0x00;
        } catch (bytes memory err) {
            return _handleError(err);
        }
    }

    function _handleReceiveLiquidity(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 status) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        try ICatalystV1Vault(toVault).receiveLiquidity(
            sourceIdentifier,                                                           // connectionId
            fromVault,                                                                  // fromVault
            address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),            // toAccount
            uint256(bytes32(data[ UNITS_START : UNITS_END ])),                          // units
            uint256(bytes32(data[ CTX1_MIN_VAULT_TOKEN_START : CTX1_MIN_VAULT_TOKEN_END ])), // minOut
            uint256(bytes32(data[ CTX1_MIN_REFERENCE_START : CTX1_MIN_REFERENCE_END ])),// minOut
            uint256(bytes32(data[ CTX1_FROM_AMOUNT_START : CTX1_FROM_AMOUNT_END ])),    // fromAmount
            uint32(bytes4(data[ CTX1_BLOCK_NUMBER_START : CTX1_BLOCK_NUMBER_END ]))     // blocknumber
        ) returns (uint256 purchasedVaultTokens) {
            uint16 dataLength = uint16(bytes2(data[CTX1_DATA_LENGTH_START:CTX1_DATA_LENGTH_END]));
            if (dataLength != 0) {
                address dataTarget = address(bytes20(data[ CTX1_DATA_START : CTX1_DATA_START+20 ]));
                bytes calldata dataArguments = data[ CTX1_DATA_START+20 : CTX1_DATA_START+dataLength ];
                
                // Let users define custom logic which should be executed after the swap.
                // The logic is not contained within a try - except so if the logic reverts
                // the transaction will timeout and the user gets the input tokens on the sending chain.
                // If this is not desired, wrap further logic in a try - except at dataTarget.
                ICatalystReceiver(dataTarget).onCatalystCall(purchasedVaultTokens, dataArguments);
                // If dataTarget doesn't implement onCatalystCall BUT implements a fallback function, the call will still succeed.
            }
            return 0x00;
        } catch (bytes memory err) {
            return _handleError(err);
        }
        
    }

    //--- Underwriting ---//
    // The following section contains the underwriting module of Catalyst.
    // It serves to speedup swap execution by letting a thirdparty take on the confirmation / delivery risk
    // by pre-executing the latter part and then reserving the swap result in the escrow.

    /**
     * @notice Returns the underwriting identifier for a Catalyst swap.
     */
    function _getUnderwriteIdentifier(
        address targetVault,
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint16 underwriteIncentiveX16,
        bytes calldata cdata
    ) internal pure returns (bytes32 identifier) {
        return identifier = keccak256(
            abi.encodePacked(
                targetVault,
                toAsset,
                U,
                minOut,
                toAccount,
                underwriteIncentiveX16,
                cdata
            )
        );
    }

    /**
     * @notice Returns the underwriting identifier for a Catalyst swap.
     */
    function getUnderwriteIdentifier(
        address targetVault,
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint16 underwriteIncentiveX16,
        bytes calldata cdata
    ) override external pure returns (bytes32 identifier) {
        return identifier = _getUnderwriteIdentifier(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            underwriteIncentiveX16,
            cdata
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
        address targetVault,  // -- Swap information
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint16 underwriteIncentiveX16,
        bytes calldata cdata
    ) override external {
        if (!ICatalystV1Vault(targetVault)._vaultConnection(sourceIdentifier, fromVault)) revert NoVaultConnection();

        underwrite(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            underwriteIncentiveX16,
            cdata
        );
    }

    /**
     * @notice Underwrite a swap.
     * There are a few important properties that underwriters should be aware of:
     * 1. There is nothing that checks if the connection is valid on this call. This check should be done seperately.
     *      Even if a swap is executed where it appears that it is possible to execute the swap,
     *      these values should be read and a connection should be checked. (via this function).
     *      As you can underwrite the swap (where there is no connection) but when the swap arrives it
     *      does not fill the underwrite but instead fails on ack and releases the input on the source chain.
     *      You can use the similar function underwriteAndCheckConnection to also check the connection.
     *
     * 2. You are underwriting the specific instance of the transaction not the inclusion of the transaction.
     *      What this means for you, is that if the block of the transaction is lost/abandon/re-entered, then
     *      the underwriting will not be noted unless the transaction is re-executed almost EXACTLY as it was before.
     *      The most important parameter is U which is volatile and any change to the vault balances on the source chain
     *      will cause U to be different.
     *      In other words, if that transaction is re-executed before or after another swap which wasn't the case before
     *      then it won't fill the underwrite anymore and either be exeucted as an ordinary swap (to the user) or fail
     *      with an ack and release the original funds back to the user.
     *
     * 3. The execution of the underwrite is dependent on the correct execution of both
     *      the minout but also the additional logic. If either fails, then the swap is not
     *      underwritable. As a result, it is important that the underwrite is simulated before executed.
     */
    function underwrite(
        address targetVault,  // -- Swap information
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint16 underwriteIncentiveX16,
        bytes calldata cdata
    ) public returns(bytes32 identifier) {

        // Get the swap identifier.
        // For any incoming swap, the swap will have a semi-unique identifier, which can be derived solely based on the
        // swap parameters. Assuming that the swap has already been executed, these arguments are constant and known.
        // As a result, once the swap arrives (and the underwriter is competent), the same identifier will be computed and matched.
        // For other implementations: The arguments the identifier is based on should be the same but the hashing
        // algorithm doesn't matter. It is only used and computed on the destination chain.
        identifier = _getUnderwriteIdentifier(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            underwriteIncentiveX16,
            cdata
        );
        // Observation: A swapper can execute multiple swaps which return the same U. 
        // In these cases, it would not be possible to underwrite the second swap (or further swaps) 
        // until after the first swap has arrived. This can be counteracted by either:
        // 1. Changing U. The tail of U is **volatile**. As a result, to get overlapping identifiers,
        // it would have to be done deliberatly.
        // 2. Add random noise to minOut or underwriteIncentiveX16. For tokens with 18 decimals, this noise can
        // be as small as 1e-12 then the chance that 2 swaps collide would be 1 in a million'th. (literally 1/(1e(18-12)) = 1/1e6)
        // 3. Add either a counter or noise to cdata.

        // For most implementations, the observation can be ignored because of the strength of point 1.

        // Check if the associated underwrite just arrived and has already been matched.
        // This is an issue when the swap was JUST underwriten, JUST arrived (and matched), AND someone else JUST underwrote the swap.
        // To give the user a bit more protection, we add a buffer of size `BUFFER_BLOCKS`.
        // SwapAlreadyUnderwritten vs SwapRecentlyUnderwritten: It is very likely that this block is trigger not because a swap was fulfilled but because it has already been underwritten. That is because (lastTouchBlock + BUFFER_BLOCKS >= uint96(block.number)) WILL ALWAYS be true when it is the case
        // and SwapRecentlyUnderwritten will be the error. You might have expected the error "SwapAlreadyUnderwritten". However, we never get there so it
        // cannot emit. We also cannot move that check up here, since then an external call would be made between a state check and a state modification. (reentry)
        // As a result, SwapRecentlyUnderwritten will be emitted when a swap has already been underwritten EXCEPT when underwriting a swap through reentry.
        // Then the reentry will underwrite the swap and the main call will fail with SwapAlreadyUnderwritten.
        unchecked {
            // Get the last touch block. For most underwrites it is going to be 0.
            uint96 lastTouchBlock = underwritingStorage[identifier].expiry;
            if (lastTouchBlock != 0) { // implies that the swap has never been underwritten.
                // if lastTouchBlock > type(uint96).max + BUFFER_BLOCKS then lastTouchBlock + BUFFER_BLOCKS overflows.
                // if lastTouchBlock < BUFFER_BLOCKS then lastTouchBlock - BUFFER_BLOCKS underflows.
                if ((lastTouchBlock <  type(uint96).max - BUFFER_BLOCKS) && (lastTouchBlock > BUFFER_BLOCKS)) {
                    // Add a reasonable buffer so if the transaction got into the memory pool and is delayed into the next blocks
                    // it doesn't underwrite a non-existing swap.
                    if (lastTouchBlock + BUFFER_BLOCKS >= uint96(block.number)) {
                        // Check that uint96(block.number) hasn't overflowed and this is an old reference. We don't care about the underflow
                        // as that will always return false.
                        // First however, we need to check that this won't underflow.
                        if (lastTouchBlock - BUFFER_BLOCKS <= uint96(block.number)) revert SwapRecentlyUnderwritten();
                    }
                } else {
                    if (lastTouchBlock == uint96(block.number)) revert SwapRecentlyUnderwritten();
                }
            }
        }

        // Get the number of purchased units from the vault. This uses a custom call which doesn't return
        // any assets.
        // This calls escrows the purchasedTokens on the vault.
        // Importantly! The connection is not checked here. Instead it is checked when the
        // message arrives. As a result, the underwriter should verify that a message is good.
        uint256 purchasedTokens = ICatalystV1Vault(targetVault).underwriteAsset(
            identifier,
            toAsset,
            U,
            minOut * (2 << 16) / ((2 << 16) - uint256(underwriteIncentiveX16))  // minout is checked after underwrite fee.
        );

        // The following number of lines act as re-entry protection. Do not add any external call inbetween these lines.

        // Ensure the swap hasn't already been underwritten by checking if refundTo is set. 
        // Notice that this is very unlikely to ever get emitted. Instead, read the comment about SwapRecentlyUnderwritten.
        if (underwritingStorage[identifier].refundTo != address(0)) revert SwapAlreadyUnderwritten();

        // Save the underwriting state.
        underwritingStorage[identifier] = UnderwritingStorage({
            tokens: purchasedTokens,
            refundTo: msg.sender,
            expiry: uint96(uint256(block.number) + uint256(maxUnderwritingDuration))  // Should never overflow.
        });

        // The above combination of lines act as local re-entry protection. Do not add any external call inbetween these lines.

        // Collect tokens and collatoral from underwriter.
        // We still collect the tokens used to incentivise the underwriter as otherwise they could freely reserve liquidity
        // in the vaults. Vaults would essentially be a free source of short term options which isn't wanted.
        ERC20(toAsset).safeTransferFrom(
            msg.sender, 
            address(this),
            purchasedTokens * (
                UNDERWRITING_COLLATORAL_DENOMINATOR+UNDERWRITING_COLLATORAL
            )/UNDERWRITING_COLLATORAL_DENOMINATOR
        );

        uint256 underwritingIncentive = (purchasedTokens * uint256(underwriteIncentiveX16)) >> 16;
        // Subtract the underwrite incentive from the funds sent to the user.
        unchecked {
            // underwritingIncentive <= purchasedTokens.
            purchasedTokens -= underwritingIncentive;
        }

        // Send the assets to the user.
        ERC20(toAsset).safeTransfer(toAccount, purchasedTokens);

        // Figure out if the user wants to execute additional logic.
        // Note that this logic is not contained within a try catch. It could fail.
        // An underwrite should simulate the tx execution before submitting the transaction as otherwise
        // they could be out the associated gas.
        uint16 calldataLength = uint16(bytes2(cdata[0:2]));
        if (calldataLength != 0) {
            address dataTarget = address(bytes20(cdata[2:2+20]));
            bytes calldata customCalldata = cdata[2+20:2+calldataLength];
            ICatalystReceiver(dataTarget).onCatalystCall(purchasedTokens, customCalldata);
        }
        
        emit SwapUnderwritten(
            identifier,
            msg.sender,
            uint96(uint256(block.number) + uint256(maxUnderwritingDuration)),
            targetVault,
            toAsset,
            U,
            toAccount,
            purchasedTokens
        );
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
        uint16 underwriteIncentiveX16,
        bytes calldata cdata
    ) override external {
        bytes32 identifier = _getUnderwriteIdentifier(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            underwriteIncentiveX16,
            cdata
        );

        UnderwritingStorage storage underwriteState = underwritingStorage[identifier];
        // Check that the refundTo address is set. (Indicates that the underwrite exists.)
        if (underwriteState.refundTo == address(0)) revert UnderwriteDoesNotExist(identifier);
        
        // Check that the underwriting can be expired. If the msg.sender is the refundTo address, then it can be expired at any time.
        // This lets the underwriter reclaim *some* of the collatoral they provided if they change their mind or observed an issue.
        if (msg.sender != underwriteState.refundTo) {
            // Otherwise, the expiry time must have been passed.
            if (underwriteState.expiry > block.number) revert UnderwriteNotExpired(underwriteState.expiry - block.number);
        }
        uint256 underWrittenTokens = underwriteState.tokens;
        // The next line acts as reentry protection. When the storage is deleted underwriteState.refundTo == address(0) will be true.
        delete underwritingStorage[identifier];

        // Delete the escrow
        ICatalystV1Vault(targetVault).deleteUnderwriteAsset(identifier, U, underWrittenTokens, toAsset);

        unchecked {
            // Compute the underwriting incentive. 
            // Notice the parts that we only have: incentive + collatoral to work with
            // The incentive was never sent to the user, neither was the underwriting incentive.
            uint256 underwritingIncentive = (underWrittenTokens * uint256(underwriteIncentiveX16)) >> 16; 
            // This computation has been done before.

            // Get the collatoral.
            // A larger computation has already been done when the swap was initially underwritten.
            uint256 refundAmount = underWrittenTokens * (
                UNDERWRITING_COLLATORAL
            )/UNDERWRITING_COLLATORAL_DENOMINATOR + underwritingIncentive;
            // collatoral + underwritingIncentive must be less than the full amount.

            // Send the coded shares of the collatoral to the expirer the rest to the vault.
        
            // This following logic might overflow but we would rather have it overflow (which reduces expireShare)
            // than to never be able to expire an underwrite.
            uint256 expireShare = refundAmount * EXPIRE_CALLER_REWARD / EXPIRE_CALLER_REWARD_DENOMINATOR;
            ERC20(toAsset).safeTransfer(msg.sender, expireShare);
            // refundAmount > expireShare, and specially when expireShare overflows.
            uint256 vaultShare = refundAmount - expireShare;
            ERC20(toAsset).safeTransfer(targetVault, vaultShare);

            emit ExpireUnderwrite(
                identifier,
                msg.sender,
                expireShare
            );
        }

        // The underwriting storage has already been deleted.
    }

    // It is important that any call to this functions has pre-checked the vault connection.
    function _matchUnderwrite(
        bytes32 identifier,
        address toAsset,
        address vault,
        bytes32 sourceIdentifier,
        bytes calldata fromVault,
        uint16 underwriteIncentiveX16
    ) internal returns (bool swapUnderwritten) {
        UnderwritingStorage storage underwriteState = underwritingStorage[identifier];
        // Load number of tokens from storage.
        uint256 underwrittenTokenAmount = underwriteState.tokens;
        // Check if the swap was underwritten => refundTo != address(0)
        address refundTo = underwriteState.refundTo;
        // if refundTo == address(0) then the swap hasn't been underwritten.
        if (refundTo == address(0)) return swapUnderwritten = false;

        // Reentry protection. No external calls are allowed before this line. The line 'if (refundTo == address(0)) ...' will always be true.
        delete underwritingStorage[identifier];
        // Set the last touch block so someone doesn't underwrite this swap again.
        underwritingStorage[identifier].expiry = uint96(block.number);

        // Delete escrow information and send swap tokens directly to the underwriter.
        ICatalystV1Vault(vault).releaseUnderwriteAsset(refundTo, identifier, underwrittenTokenAmount, toAsset, sourceIdentifier, fromVault);
        // We know only need to handle the collatoral and underwriting incentive.
        // We also don't have to check that the vault didn't lie to us about underwriting.

        // Also refund the collatoral.
        uint256 refundAmount = underwrittenTokenAmount * (
            UNDERWRITING_COLLATORAL
        )/UNDERWRITING_COLLATORAL_DENOMINATOR;

        // add the underwriting incentive as well. Notice that 2x refundAmount are in play.
        //   1. The first part comes from the underwriter + collatoral.
        // + 1. The second part comes from the vault after the matching message arrives.
        // = 2 parts
        // 1 - incentive has been sent to the user.
        // That leaves us with part (1 - (1 - incentive)) + (1) = incentive + 1
        uint256 underwritingIncentive = (underwrittenTokenAmount * uint256(underwriteIncentiveX16)) >> 16;
        refundAmount += underwritingIncentive;

        ERC20(toAsset).safeTransfer(refundTo, refundAmount);

        emit FulfillUnderwrite(
            identifier
        );

        return swapUnderwritten = true;
    }

    function _handleReceiveAsset(bytes32 sourceIdentifier, bytes calldata data) internal returns (bytes1 acknowledgement) {
        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        // Select excess calldata. Excess calldata is not decoded.
        bytes calldata cdata = data[CTX0_DATA_LENGTH_START:];

        // Get the toAsset
        uint8 toAssetIndex = uint8(data[CTX0_TO_ASSET_INDEX_POS]);
        address toAsset = ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex);

        // Get the rest of the swap parameters.
        address toAccount = address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]));
        uint256 U = uint256(bytes32(data[ UNITS_START : UNITS_END ]));
        uint256 minOut = uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ]));
        uint16 underwriteIncentiveX16 = uint16(bytes2(data[CTX0_UW_INCENTIVE_START:CTX0_UW_INCENTIVE_END]));

        // Get the underwriting identifier.
        bytes32 identifier = _getUnderwriteIdentifier(
            toVault,
            toAsset,
            U,
            minOut,
            toAccount,
            underwriteIncentiveX16,
            cdata
        );

        // Check if the swap has been underwritten. If it has, return funds to underwriter.
        bool swapUnderwritten = _matchUnderwrite(
            identifier,
            toAsset,
            toVault,
            sourceIdentifier,
            fromVault,
            underwriteIncentiveX16
        );

        if (!swapUnderwritten) {
            // The swap hasn't been underwritten lets execute the swap properly.
            return acknowledgement = _handleReceiveAssetFallback(sourceIdentifier, data);
        }
        // There is no case where only a subset of the units are filled. As either the complete swap (through the swap identifier)
        // is underwritten or it wasn't underwritten.

        return acknowledgement = 0x00;
    }
}
