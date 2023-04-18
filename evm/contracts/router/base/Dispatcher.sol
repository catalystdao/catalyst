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
            // 0x00 <= command < 0x08
            if (command < 0x07) {
                if (command == Commands.LOCALSWAP) {
                    // equivalent:  abi.decode(inputs, (address, address, address, uint256, uint256))
                    address pool;
                    address fromAsset;
                    address toAsset;
                    uint256 amount;
                    uint256 minOut;
                    assembly {
                        pool := calldataload(inputs.offset)
                        fromAsset := calldataload(add(inputs.offset, 0x20))
                        toAsset := calldataload(add(inputs.offset, 0x40))
                        amount := calldataload(add(inputs.offset, 0x60))
                        minOut := calldataload(add(inputs.offset, 0x80))
                    }
                    CatalystExchange.localSwap(pool, fromAsset, toAsset, amount, minOut);
                }  else if (command == Commands.SENDASSET) {
                    // equivalent: abi.decode(inputs, (address, bytes32, bytes32, bytes32, address, uint256, uint256, uint256, address, bytes))
                    address pool;
                    bytes32 channelId;
                    bytes32 targetPool;
                    bytes32 targetUser;
                    address fromAsset;
                    uint256 toAssetIndex256;
                    uint256 amount;
                    uint256 minOut;
                    address fallbackUser;
                    assembly {
                        pool := calldataload(inputs.offset)
                        channelId := calldataload(add(inputs.offset, 0x20))
                        targetPool := calldataload(add(inputs.offset, 0x40))
                        targetUser := calldataload(add(inputs.offset, 0x60))
                        fromAsset := calldataload(add(inputs.offset, 0x80))
                        toAssetIndex256 := calldataload(add(inputs.offset, 0xa0))
                        amount := calldataload(add(inputs.offset, 0xc0))
                        minOut := calldataload(add(inputs.offset, 0xe0))
                        fallbackUser := calldataload(add(inputs.offset, 0x100))
                    }

                    bytes calldata calldata_ = inputs[0x120:];

                    uint8 toAssetIndex8 = uint8(toAssetIndex256);
                    
                    CatalystExchange.sendAsset(pool, channelId, targetPool, targetUser, fromAsset, toAssetIndex8, amount, minOut, fallbackUser, calldata_);
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
            } else {
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
                    address pool;
                    uint256 amount;
                    assembly {
                        pool := calldataload(inputs.offset)
                        amount := calldataload(add(inputs.offset, 0x20))
                    }

                    uint256[] calldata minOut = inputs.toUintArray(2);
                    
                    CatalystExchange.withdrawAll(pool, amount, minOut);
                } else if (command == Commands.WITHDRAW_MIXED) {
                    // equivalent:  abi.decode(inputs, (address, uint256, uint256[], uint256[]))
                    address pool;
                    uint256 amount;
                    assembly {
                        pool := calldataload(inputs.offset)
                        amount := calldataload(add(inputs.offset, 0x20))
                    }

                    uint256[] calldata withdrawRatio = inputs.toUintArray(2);
                    uint256[] calldata minOut = inputs.toUintArray(3);

                    CatalystExchange.withdrawMixed(pool, amount, withdrawRatio, minOut);
                } else if (command == Commands.DEPOSIT_MIXED) {
                    // equivalent:  abi.decode(inputs, (address, address[], uint256[], uint256))
                    address pool;
                    uint256 minOut;
                    assembly {
                        pool := calldataload(inputs.offset)
                        minOut := calldataload(add(inputs.offset, 0x60))
                    }

                    address[] calldata tokens = inputs.toAddressArray(1);
                    uint256[] calldata tokenAmounts = inputs.toUintArray(2);

                    CatalystExchange.depositMixed(pool, tokens, tokenAmounts, minOut);
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
            if (command == Commands.EXECUTE_SUB_PLAN) {
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