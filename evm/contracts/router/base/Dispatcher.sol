// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.16;

import {IAllowanceTransfer} from 'lib/permit2/src/interfaces/IAllowanceTransfer.sol';
import {ERC20} from 'lib/solmate/src/tokens/ERC20.sol';
import {RouterImmutables} from '../base/RouterImmutables.sol';
import {Payments} from '../libraries/Payments.sol';
import {Permit2Payments} from '../libraries/Permit2Payments.sol';
import {CatalystExchange} from '../libraries/CatalystExchange.sol';
import {Commands} from '../libraries/Commands.sol';
import {BytesLib} from '../libraries/BytesLib.sol';
import {CancelSwap} from '../libraries/CancelSwap.sol';
import {LockAndMsgSender} from './LockAndMsgSender.sol';

/// @title Decodes and Executes Commands
/// @notice Called by the UniversalRouter contract to efficiently decode and execute a singular command
abstract contract Dispatcher is Permit2Payments, CatalystExchange, CancelSwap, LockAndMsgSender {
    using BytesLib for bytes;

    error debugError(bytes tt);
    error InvalidCommandType(uint256 commandType);
    error BuyPunkFailed();
    error InvalidOwnerERC721();
    error InvalidOwnerERC1155();
    error BalanceTooLow();

    /// @notice Decodes and executes the given command with the given inputs
    /// @param commandType The command type to execute
    /// @param inputs The inputs to execute the command with
    /// @dev 2 masks are used to enable use of a nested-if statement in execution for efficiency reasons
    /// @return success True on success of the command, false on failure
    /// @return output The outputs or error messages, if any, from the command
    function dispatch(bytes1 commandType, bytes calldata inputs) internal returns (bool success, bytes memory output) {
        uint256 command = uint8(commandType & Commands.COMMAND_TYPE_MASK);

        success = true;

        if (command < 0x10) {
            // 0x00 <= command < 0x07
            if (command < 0x07) {
                if (command == Commands.LOCALSWAP) {
                    // equivalent:  abi.decode(inputs, (address, address, address, uint256, uint256))
                    address vault;
                    address fromAsset;
                    address toAsset;
                    uint256 amount;
                    uint256 minOut;
                    assembly {
                        vault := calldataload(inputs.offset)
                        fromAsset := calldataload(add(inputs.offset, 0x20))
                        toAsset := calldataload(add(inputs.offset, 0x40))
                        amount := calldataload(add(inputs.offset, 0x60))
                        minOut := calldataload(add(inputs.offset, 0x80))
                    }
                    CatalystExchange.localSwap(vault, fromAsset, toAsset, amount, minOut);
                }  else if (command == Commands.SENDASSET) {
                    // equivalent: abi.decode(inputs, (address, bytes32, bytes, bytes, address, uint256, uint256, uint256, address))
                    // address vault;
                    // bytes32 channelId;
                    // bytes calldata toVault = inputs.toBytes(0x40);
                    // bytes calldata toUser = inputs.toBytes(0x60);
                    // address fromAsset;
                    // uint256 toAssetIndex256;
                    // uint256 amount;
                    // uint256 minOut;
                    // address fallbackUser;
                    // assembly {
                    //     vault := calldataload(inputs.offset)
                    //     channelId := calldataload(add(inputs.offset, 0x20))
                    //     // toVault := calldataload(add(inputs.offset, 0x40))
                    //     // toUser := calldataload(add(inputs.offset, 0x60))
                    //     fromAsset := calldataload(add(inputs.offset, 0x80))
                    //     toAssetIndex256 := calldataload(add(inputs.offset, 0xa0))
                    //     amount := calldataload(add(inputs.offset, 0xc0))
                    //     minOut := calldataload(add(inputs.offset, 0xe0))
                    //     fallbackUser := calldataload(add(inputs.offset, 0x100))
                    // }

                    // TODO: Decode memory bytes in calldata. See above.
                    (address vault, bytes32 channelId, bytes memory toVault, bytes memory toUser, address fromAsset, uint8 toAssetIndex8, uint256 amount, uint256 minOut, address fallbackUser) = abi.decode(inputs, (address, bytes32, bytes, bytes, address, uint8, uint256, uint256, address));

                    // We don't have space in the stack do dynamically decode the calldata. 
                    // To circumvent that, we have to decode it as a slice. We need to start after
                    // all initial variables (ends at 0x120) AND after the dynamically decoded bytes.
                    // Luckly, we know that toVault and toUser is 65 bytes long. With 2 length indicators of 32 bytes
                    // Calldata must be everything after: 0x120 + 65 * 2 + (32*3-65) * 2 + 2 * 32 = 544 = 0x220.
                    bytes calldata calldata_ = inputs[0x220:];
                    
                    CatalystExchange.sendAsset(vault, channelId, toVault, toUser, fromAsset, toAssetIndex8, amount, minOut, map(fallbackUser), calldata_);
                } else if (command == Commands.PERMIT2_TRANSFER_FROM) {
                    // equivalent: abi.decode(inputs, (address, address, uint160))
                    address token;
                    address recipient;
                    uint160 amount;
                    assembly {
                        token := calldataload(inputs.offset)
                        recipient := calldataload(add(inputs.offset, 0x20))
                        amount := calldataload(add(inputs.offset, 0x40))
                    }
                    permit2TransferFrom(token, lockedBy, map(recipient), amount);
                } else if (command == Commands.PERMIT2_PERMIT_BATCH) {
                    (IAllowanceTransfer.PermitBatch memory permitBatch,) =
                        abi.decode(inputs, (IAllowanceTransfer.PermitBatch, bytes));
                    bytes calldata data = inputs.toBytes(1);
                    PERMIT2.permit(lockedBy, permitBatch, data);
                } else if (command == Commands.SWEEP) {
                    // equivalent:  abi.decode(inputs, (address, address, uint256))
                    address token;
                    address recipient;
                    uint160 amountMin;
                    assembly {
                        token := calldataload(inputs.offset)
                        recipient := calldataload(add(inputs.offset, 0x20))
                        amountMin := calldataload(add(inputs.offset, 0x40))
                    }
                    Payments.sweep(token, map(recipient), amountMin);
                } else if (command == Commands.TRANSFER) {
                    // equivalent:  abi.decode(inputs, (address, address, uint256))
                    address token;
                    address recipient;
                    uint256 value;
                    assembly {
                        token := calldataload(inputs.offset)
                        recipient := calldataload(add(inputs.offset, 0x20))
                        value := calldataload(add(inputs.offset, 0x40))
                    }
                    Payments.pay(token, map(recipient), value);
                } else if (command == Commands.PAY_PORTION) {
                    // equivalent:  abi.decode(inputs, (address, address, uint256))
                    address token;
                    address recipient;
                    uint256 bips;
                    assembly {
                        token := calldataload(inputs.offset)
                        recipient := calldataload(add(inputs.offset, 0x20))
                        bips := calldataload(add(inputs.offset, 0x40))
                    }
                    Payments.payPortion(token, map(recipient), bips);
                }
            // 0x08 <= command < 0x0d
            } else if (command < 0x0d) { // 0x08 <= command < 0x0d
                if (command == Commands.PERMIT2_PERMIT) {
                    // equivalent: abi.decode(inputs, (IAllowanceTransfer.PermitSingle, bytes))
                    IAllowanceTransfer.PermitSingle calldata permitSingle;
                    assembly {
                        permitSingle := inputs.offset
                    }
                    bytes calldata data = inputs.toBytes(6); // PermitSingle takes first 6 slots (0..5)
                    PERMIT2.permit(lockedBy, permitSingle, data);
                } else if (command == Commands.WRAP_GAS) {
                    // equivalent: abi.decode(inputs, (address, uint256))
                    address recipient;
                    uint256 amountMin;
                    assembly {
                        recipient := calldataload(inputs.offset)
                        amountMin := calldataload(add(inputs.offset, 0x20))
                    }
                    Payments.wrapETH(map(recipient), amountMin);
                } else if (command == Commands.UNWRAP_GAS) {
                    // equivalent: abi.decode(inputs, (address, uint256))
                    address recipient;
                    uint256 amountMin;
                    assembly {
                        recipient := calldataload(inputs.offset)
                        amountMin := calldataload(add(inputs.offset, 0x20))
                    }
                    Payments.unwrapWETH9(map(recipient), amountMin);
                } else if (command == Commands.WITHDRAW_EQUAL) {
                    // equivalent:  abi.decode(inputs, (address, uint256, uint256[]))
                    address vault;
                    uint256 amount;
                    assembly {
                        vault := calldataload(inputs.offset)
                        amount := calldataload(add(inputs.offset, 0x20))
                    }

                    uint256[] calldata minOut = inputs.toUintArray(2);
                    
                    CatalystExchange.withdrawAll(vault, amount, minOut);
                } else if (command == Commands.WITHDRAW_MIXED) {
                    // equivalent:  abi.decode(inputs, (address, uint256, uint256[], uint256[]))
                    address vault;
                    uint256 amount;
                    assembly {
                        vault := calldataload(inputs.offset)
                        amount := calldataload(add(inputs.offset, 0x20))
                    }

                    uint256[] calldata withdrawRatio = inputs.toUintArray(2);
                    uint256[] calldata minOut = inputs.toUintArray(3);

                    CatalystExchange.withdrawMixed(vault, amount, withdrawRatio, minOut);
                } else if (command == Commands.DEPOSIT_MIXED) {
                    // equivalent:  abi.decode(inputs, (address, address[], uint256[], uint256))
                    address vault;
                    uint256 minOut;
                    assembly {
                        vault := calldataload(inputs.offset)
                        minOut := calldataload(add(inputs.offset, 0x60))
                    }

                    address[] calldata tokens = inputs.toAddressArray(1);
                    uint256[] calldata tokenAmounts = inputs.toUintArray(2);

                    CatalystExchange.depositMixed(vault, tokens, tokenAmounts, minOut);
                }
                // 0x0e <= command < 0x10
            } else if (command < 0x10) {
                if (command == Commands.SENDLIQUIDITY) {
                    // TODO: Decode memory variables in calldata. See sendAsset.
                    (address vault, bytes32 channelId, bytes memory toVault, bytes memory toUser, address fromAsset, uint256 amount, uint256[2] memory minOut, address fallbackUser) = abi.decode(inputs, (address, bytes32, bytes, bytes, address, uint256, uint256[2], address));

                    // We don't have space in the stack do dynamically decode the calldata. 
                    // To circumvent that, we have to decode it as a slice. We need to start after
                    // all initial variables (ends at 0x120) AND after the dynamically decoded bytes.
                    // Luckly, we know that toVault and toUser is 65 bytes long. With 2 length indicators of 32 bytes
                    // Calldata must be everything after: 0x120 + 65 * 2 + (32*3-65) * 2 + 2 * 32 = 544 = 0x220.
                    bytes calldata calldata_ = inputs[0x220:];
                    
                    CatalystExchange.sendLiquidity(vault, channelId, toVault, toUser, amount, minOut, fallbackUser, calldata_);
                } else if (command == Commands.ALLOW_CANCEL) {
                    // equivalent: abi.decode(inputs, (address, bytes32))
                    address swappie;
                    bytes32 cancelIdentifier;
                    assembly {
                        swappie := calldataload(inputs.offset)
                        cancelIdentifier := calldataload(add(inputs.offset, 0x20))
                    }
                    CancelSwap.requireNotCanceled(swappie, cancelIdentifier);
                } else if (command == Commands.BALANCE_CHECK_ERC20) {
                    // equivalent: abi.decode(inputs, (address, address, uint256))
                    address owner;
                    address token;
                    uint256 minBalance;
                    assembly {
                        owner := calldataload(inputs.offset)
                        token := calldataload(add(inputs.offset, 0x20))
                        minBalance := calldataload(add(inputs.offset, 0x40))
                    }
                    success = (ERC20(token).balanceOf(owner) >= minBalance);
                    if (!success) output = abi.encodePacked(BalanceTooLow.selector);
                }
            }
        } else {
             if (command == Commands.TRANSFER_FROM) {
                    // equivalent: abi.decode(inputs, (address, address, uint160))
                    address token;
                    address recipient;
                    uint160 amount;
                    assembly {
                        token := calldataload(inputs.offset)
                        recipient := calldataload(add(inputs.offset, 0x20))
                        amount := calldataload(add(inputs.offset, 0x40))
                    }
                    Payments.transferFrom(token, lockedBy, map(recipient), amount);
            } else if (command == Commands.EXECUTE_SUB_PLAN) {
                (bytes memory _commands, bytes[] memory _inputs) = abi.decode(inputs, (bytes, bytes[]));
                (success, output) =
                    (address(this)).call(abi.encodeWithSelector(Dispatcher.execute.selector, _commands, _inputs));
            } else {
                // placeholder area for all other commands
                revert InvalidCommandType(command);
            }
        }
    }

    /// @notice Executes encoded commands along with provided inputs.
    /// @param commands A set of concatenated commands, each 1 byte in length
    /// @param inputs An array of byte strings containing abi encoded inputs for each command
    function execute(bytes calldata commands, bytes[] calldata inputs) external payable virtual;
}