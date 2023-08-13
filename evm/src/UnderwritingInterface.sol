//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import { ICatalystReceiver } from "./interfaces/IOnCatalyst.sol";
import { CatalystGARPInterface } from "./CatalystGARPInterface.sol";
import { ICatalystV1Vault } from "./ICatalystV1Vault.sol";
import "./utils/FixedPointMathLib.sol";
import { CTX2_ASSET_SWAP_PLEASE_UNDERWRITE, CTX3_ASSET_SWAP_PURPOSE_UNDERWRITE } from "./CatalystPayload.sol";
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
contract UnderwritingInterface is CatalystGARPInterface {
    using SafeTransferLib for ERC20;

    uint256 constant public UNDERWRITING_UNFULFILLED_FEE = 1035;  // 3,5% extra as collatoral.
    uint256 constant public UNDERWRITING_UNFULFILLED_FEE_DENOMINATOR = 1000;

    uint256 constant public EXPIRE_CALLER_REWARD = 35;  // 35% of the 3,5% = 1,225%. Of $1000 = $12,25
    uint256 constant public EXPIRE_CALLER_REWARD_DENOMINATOR = 100;

    error SwapError(bytes1 errorCode);
    error SwapAlreadyUnderwritten();
    error RefundToZeroAddress();
    error UnderwriteDoesNotExist(bytes32 identifier);
    error UnderwriteNotExpired(uint256 timeUnitilExpiry);
    error MaxUnderwriteDurationTooLong();

    struct UnderwritingStorage {
        uint256 tokens;     // 1 slot
        address refundTo;   // 2 slot: 20/32
        uint80 expiry;      // 2 slot: 30/32
    }

    /**
     * @dev The collatoral and thus the reward for expiring an underwrite can be derived through fromAmount. 
     * All the arguments to derive the identifier can be found within event. This reduces the complexity for clients
     * calling expired underwrites.
     */
    event UnderwriteSwap(
        bytes32 indexed identifier,
        address indexed underwriter,
        uint80 expiry,
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

    /// @notice Sets the maximum duration for underwriting.
    /// @dev Should be set long enough for all swaps to be able to confirm + a small buffer
    /// Should also be set long enough to not take up an excess amount of escrow usage.
    uint256 public maxUnderwritingDuration = 12 hours;

    mapping(bytes32 => UnderwritingStorage) public underwritingStorage;

    constructor(address GARP_) CatalystGARPInterface(GARP_) {
    }

    // TODO: only owner
    function setMaxUnderwritingDuration(uint256 newMaxUnderwriteDuration) external {
        // If the underwriting duration is too long, users can freeze up a lot of value for not a lot of cost.
        if (newMaxUnderwriteDuration > 15 days) revert MaxUnderwriteDurationTooLong();

        maxUnderwritingDuration = newMaxUnderwriteDuration;
    }

    function _receiveMessage(bytes32 sourceIdentifierbytes, bytes calldata data) internal override returns (bytes1 acknowledgement) {
        acknowledgement = super._receiveMessage(sourceIdentifierbytes, data);
        // If acknowledgement is 0x01, it implies InvalidContext. Since this contract extends the
        // underlying contract, it is unaware of these "new" contexts.
        // As a result, we should not handle any acks which are not 0x01.
        if (acknowledgement != 0x01) {
            // This message has been handled by the underlying implementation. We shouldn't handle these.
            return acknowledgement;
        }
        // Get context so we can figure out what type a message is.
        bytes1 context = data[0];
        if (context == CTX2_ASSET_SWAP_PLEASE_UNDERWRITE) {
            return acknowledgement = _pleaseFill(sourceIdentifierbytes, data);
        }
        if (context == CTX3_ASSET_SWAP_PURPOSE_UNDERWRITE) {
            return acknowledgement = _purposeFill(sourceIdentifierbytes, data);
        }
    }

    function getSwapIdentifier(
        address targetVault,
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
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
                cdata
            )
        );
    }

    function underwrite(
        address refundTo,     // -- Underwriting State
        address targetVault,  // -- Swap information
        address toAsset,
        uint256 U,
        uint256 minOut,
        address toAccount,
        uint256 fromAmount,
        bytes calldata cdata
    ) external {
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
        bytes calldata cdata
    ) external {
        bytes32 identifier = getSwapIdentifier(
            targetVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
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
        uint256 refundAmount = underwriteState.tokens * (
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
        // TODO: alt just fill it by the units collected? If the swap finally fails, then let the 
        // underwrite execute a swap of x units.
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

    function _pleaseFill(bytes32 sourceIdentifierbytes, bytes calldata data) internal returns (bytes1 acknowledgement) {
        // 1. Generate the underwriting ID.

        // We don't know how from_vault is encoded. So we load it as bytes. Including the length.
        bytes calldata fromVault = data[ FROM_VAULT_LENGTH_POS : FROM_VAULT_END ];
        // We know that toVault is an EVM address
        address toVault = address(bytes20(data[ TO_VAULT_START_EVM : TO_VAULT_END ]));

        // uint16 dataLength = uint16(bytes2(data[CTX0_DATA_LENGTH_START:CTX0_DATA_LENGTH_END]));

        // Select excess calldata. Excess calldata is not decoded.
        bytes calldata cdata = data[CTX0_DATA_LENGTH_START:];

        // Get the toAsset
        uint8 toAssetIndex = uint8(data[CTX0_TO_ASSET_INDEX_POS]);
        address toAsset = ICatalystV1Vault(toVault)._tokenIndexing(toAssetIndex);

        // Get the rest of the swap parameters.
        address toAccount = address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ]));
        uint256 U = uint256(bytes32(data[ UNITS_START : UNITS_END ]));
        uint256 minOut = uint256(bytes32(data[ CTX0_MIN_OUT_START : CTX0_MIN_OUT_END ]));
        uint256 fromAmount = uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ]));

        // Get the underwriting identifier.
        bytes32 identifier = getSwapIdentifier(
            toVault,
            toAsset,
            U,
            minOut,
            toAccount,
            fromAmount,
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
            return acknowledgement = _handleOrdinarySwap(sourceIdentifierbytes, data);
        }
        if (unusedUnits > 0) {
            // Use unspent units to purchase tokens. This is fallback logic. The payload won't be executed again.
            // TODO: Fallback account / fallback calldata?
            try ICatalystV1Vault(toVault).receiveAsset(
                sourceIdentifierbytes,                                                      // connectionId
                fromVault,                                                                   // fromVault
                uint8(data[CTX0_TO_ASSET_INDEX_POS]),                                       // toAssetIndex
                address(bytes20(data[ TO_ACCOUNT_START_EVM : TO_ACCOUNT_END ])),           // toAccount
                uint256(unusedUnits),                                                       // units
                uint256(0),                                                                 // minOut
                uint256(bytes32(data[ CTX0_FROM_AMOUNT_START : CTX0_FROM_AMOUNT_END ])),    // fromAmount
                bytes(data[ CTX0_FROM_ASSET_LENGTH_POS : CTX0_FROM_ASSET_END ]),            // fromAsset
                uint32(bytes4(data[ CTX0_BLOCK_NUMBER_START : CTX0_BLOCK_NUMBER_END ]))    // block number
            ) {
                // Return outside try catch to save a bit of deployment gas.
            } catch (bytes memory err) {
                // Since the swap already properly executed, we shouldn't have to do anything extra.
                // Instead, the best fallback is to sacrafiise the unspent units.
            }
            return acknowledgement = 0x00;
        }

        return acknowledgement = 0x00;
    }

    // This function is used to pre-specify the identifier to backfill underwritten swaps.
    // While _pleaseFill could technically be used for it, it can't because the identifier contains U. 
    function _purposeFill(bytes32 sourceIdentifierbytes, bytes calldata message) internal returns (bytes1 acknowledgement) {
        // TODO:
        // 1. Get underwriting ID.

        // 2. Check if swap has been underwritten underwritten.
        // 3. Fulfill underwritten position.
        // 3.b. Execute fallback (aka context CTX0)
        return 0x00;
    }

}
