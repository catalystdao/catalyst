// SPDX-License-Identifier: GPL-3.0-or-later

/// @title Library for Bytes Manipulation
/// Based on Gonçalo Sá's BytesLib - but updated and heavily editted
pragma solidity ^0.8.0;

library BytesLib {
    /// @notice Returns the address starting at byte 0
    /// @dev length and overflow checks must be carried out before calling
    /// @param _bytes The input bytes string to slice
    /// @return tempAddress The address starting at byte 0
    function toAddress(bytes calldata _bytes) internal pure returns (address tempAddress) {
        assembly {
            tempAddress := shr(96, calldataload(_bytes.offset))
        }
    }

    function toBytes(bytes calldata _bytes, uint256 arg) internal pure returns (bytes calldata res) {
        assembly {
            let lengthPtr := add(_bytes.offset, calldataload(add(_bytes.offset, mul(0x20, arg))))
            res.offset := add(lengthPtr, 0x20)
            res.length := calldataload(lengthPtr)
        }
    }

    function toBytesArray(bytes calldata _bytes, uint256 arg) internal pure returns (bytes[] calldata res) {
        assembly {
            let lengthPtr := add(_bytes.offset, calldataload(add(_bytes.offset, mul(0x20, arg))))
            res.offset := add(lengthPtr, 0x20)
            res.length := calldataload(lengthPtr)
        }
    }

    function toAddressArray(bytes calldata _bytes, uint256 arg) internal pure returns (address[] calldata res) {
        assembly {
            let lengthPtr := add(_bytes.offset, calldataload(add(_bytes.offset, mul(0x20, arg))))
            res.offset := add(lengthPtr, 0x20)
            res.length := calldataload(lengthPtr)
        }
    }

    function toUintArray(bytes calldata _bytes, uint256 arg) internal pure returns (uint256[] calldata res) {
        assembly {
            let lengthPtr := add(_bytes.offset, calldataload(add(_bytes.offset, mul(0x20, arg))))
            res.offset := add(lengthPtr, 0x20)
            res.length := calldataload(lengthPtr)
        }
    }
}