//SPDX-License-Identifier: Unlicsened
pragma solidity ^0.8.17;

/// @title Administrative actions defined by Catalyst v1 Pools
/// @notice Contains all functions which can only be called by privileged users.
interface ICatalystV1PoolAdministration {
    function setFeeAdministrator(address newFeeAdministrator) external;

    function setPoolFee(uint256 newPoolFeeX64) external;

    /**
     * @notice Creates a connection to the pool _poolReceiving on the channel _channelId.
     * @dev if _poolReceiving is an EVM pool, it can be computes as:
     *     Vyper: convert(_poolAddress, bytes32)
     *     Solidity: abi.encode(_poolAddress)
     *     Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * ! Notice, using tx.origin is not secure.
     * However, it makes it easy to bundle call from an external contract
     * and no assets are at risk because the pool should not be used without
     * setupMaster == ZERO_ADDRESS
     * @param channelId The _channelId of the target pool.
     * @param poolReceiving The bytes32 representation of the target pool
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function createConnection(bytes32 channelId, bytes32 poolReceiving, bool state) external;

    /**
     * @notice Gives up short term ownership of the pool. This makes the pool unstoppable.
     * @dev ! Using tx.origin is not secure.
     * However, it makes it easy to bundle call from an external contract
     * and no assets are at risk because the pool should not be used without
     * setupMaster == ZERO_ADDRESS
     */
    function finishSetup() external;
}
