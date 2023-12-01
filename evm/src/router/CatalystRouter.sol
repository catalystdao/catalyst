//SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.19;

// Command implementations
import {Dispatcher} from './base/Dispatcher.sol';
import {RouterParameters, RouterImmutables} from './base/RouterImmutables.sol';
import {Commands} from './libraries/Commands.sol';
import {BytesLib} from './libraries/BytesLib.sol';
import {ICatalystRouter} from './interfaces/ICatalystRouter.sol';
import {ICatalystReceiver} from '../interfaces/IOnCatalyst.sol';

/**
 * @title Catalyst: Swap Router
 * @author Catalyst Labs
 * @notice Based on the Universal Router by Uniswap
 * https://github.com/Uniswap/universal-router
 */
contract CatalystRouter is RouterImmutables, ICatalystRouter, Dispatcher, ICatalystReceiver {
    using BytesLib for bytes;

    modifier checkDeadline(uint256 deadline) {
        if (block.timestamp > deadline) revert TransactionDeadlinePassed();
        _;
    }

    constructor(RouterParameters memory params) RouterImmutables(params) {}

    /// @inheritdoc ICatalystRouter
    function execute(
        bytes calldata commands,
        bytes[] calldata inputs,
        uint256 deadline
    ) external payable checkDeadline(deadline) {
        execute(commands, inputs);
    }

    /// @inheritdoc Dispatcher
    function execute(bytes calldata commands, bytes[] calldata inputs)
        public
        payable
        override
        isNotLocked
    {
        bool success;
        bytes memory output;
        uint256 numCommands = commands.length;
        if (inputs.length != numCommands) revert LengthMismatch();

        // loop through all given commands, execute them and pass along outputs as defined
        for (uint256 commandIndex = 0; commandIndex < numCommands; ) {
            bytes1 command = commands[commandIndex];

            bytes calldata input = inputs[commandIndex];

            (success, output) = dispatch(command, input);

            if (!success && successRequired(command)) {
                revert ExecutionFailed({
                    commandIndex: commandIndex,
                    message: output
                });
            }

            unchecked {
                commandIndex++;
            }
        }
    }

    function successRequired(bytes1 command) internal pure returns (bool) {
        return command & Commands.FLAG_ALLOW_REVERT == 0;
    }

    function onCatalystCall(uint256 purchasedTokens, bytes calldata data) external {
        bytes calldata commands = data.toBytes(0);
        bytes[] calldata inputs = data.toBytesArray(1);

        execute(commands, inputs);
    }

    /// @notice To receive ETH from WETH and NFT protocols
    receive() external payable {}
}
