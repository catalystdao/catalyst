// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

interface ICatalystRouter {
    /// @notice Thrown when a required command has failed
    error ExecutionFailed(uint256 commandIndex, bytes message);  // 2c4029e9

    /// @notice Thrown when attempting to send ETH directly to the contract
    error ETHNotAccepted();  // 1231ae4

    /// @notice Thrown when executing commands with an expired deadline
    error TransactionDeadlinePassed();  // 5bf6f916

    /// @notice Thrown when attempting to execute commands and an incorrect number of inputs are provided
    error LengthMismatch();  // ff633a38

    /// @notice Executes encoded commands along with provided inputs. Reverts if deadline has expired.
    /// @param commands A set of concatenated commands, each 1 byte in length
    /// @param inputs An array of byte strings containing abi encoded inputs for each command
    /// @param deadline The deadline by which the transaction must be executed
    function execute(bytes calldata commands, bytes[] calldata inputs, uint256 deadline) external payable;
}