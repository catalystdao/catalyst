// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

/// @title Cancel on-going swaps
/// @notice Allows swappers to cancel their swaps. Given that they already have gas and access to the target chain.
abstract contract CancelSwap {
    error SwapCanceled();

    mapping(address => mapping(bytes32 => bool)) public swapIdentifier;

    /// @notice Check if a swap has been cancled
    /// @dev Because swappie is not given to be the swapper, it can be an external contract
    /// which allows people to cancel swaps even from the sending chain. (given high routing incentives)
    /// This contract could then encode cancelIdentifier as a hash of the swapper and another iterator:
    ///     keccak256(abi.encodePacked(swapper, block.timestamp))
    /// @param swappie The address to check for cancelment
    /// @param cancelIdentifier The token to pay (can be ETH using Constants.ETH)
    function requireNotCanceled(address swappie, bytes32 cancelIdentifier) internal view {
        if (!swapIdentifier[swappie][cancelIdentifier]) revert SwapCanceled();
    }

    /// @notice If an ognoing swap has cancelIdentifier, calling this function will cancel the swap
    /// @param cancelIdentifier The swap identifier to set.
    function cancelSwap(bytes32 cancelIdentifier) external {
        swapIdentifier[msg.sender][cancelIdentifier] = true;
    }

    /// @notice If an ognoing swap has cancelIdentifier, calling this function will cancel the swap
    /// @param cancelIdentifier The swap identifier to set.
    /// @param state Sets a new state for a specific identifier. Allows one to uncancel a swap.
    function cancelSwap(bytes32 cancelIdentifier, bool state) external {
        swapIdentifier[msg.sender][cancelIdentifier] = state;
    }
}