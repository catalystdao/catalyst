// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.16;

import {Payments} from '../libraries/Payments.sol';
import {RouterImmutables} from '../base/RouterImmutables.sol';
import {Commands} from '../libraries/Commands.sol';
import {LockAndMsgSender} from './LockAndMsgSender.sol';

/// @title Decodes and Executes Commands
/// @notice Called by the UniversalRouter contract to efficiently decode and execute a singular command
abstract contract Dispatcher is Payments, LockAndMsgSender {

    error InvalidCommandType(uint256 commandType);
    error BuyPunkFailed();
    error InvalidOwnerERC721();
    error InvalidOwnerERC1155();

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
            if (command < 0x08) {
                if (command == Commands.LOCALSWAP) {
                    // todo
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
                } else if (command == Commands.TRANSFER_PORTION) {
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
                } else if (command == Commands.PERMIT) {
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
                } else {
                    // placeholder area for command 0x07
                    revert InvalidCommandType(command);
                }
                // 0x08 <= command < 0x10
            } else {
                if (command == Commands.SENDSWAP) {
                    // todo
                } else if (command == Commands.WITHDRAW_EQUAL) {
                    // todo
                } else if (command == Commands.WITHDRAW_MIXED) {
                    // todo
                } else if (command == Commands.DEPOSIT_MIXED) {
                    // todo
                } else {
                    // placeholder area for commands 0x0e-0x0f
                    revert InvalidCommandType(command);
                }
            }
        } else {
            if (command == Commands.EXECUTE_SUB_PLAN) {
                (bytes memory _commands, bytes[] memory _inputs) = abi.decode(inputs, (bytes, bytes[]));
                (success, output) =
                    (address(this)).call(abi.encodeWithSelector(Dispatcher.execute.selector, _commands, _inputs));
            } else {
                // placeholder area for commands 0x21-0x3f
                revert InvalidCommandType(command);
            }
        }
    }

    /// @notice Executes encoded commands along with provided inputs.
    /// @param commands A set of concatenated commands, each 1 byte in length
    /// @param inputs An array of byte strings containing abi encoded inputs for each command
    function execute(bytes calldata commands, bytes[] calldata inputs) external payable virtual;
}