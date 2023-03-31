//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

/// @title Administrative actions defined by Catalyst v1 Pools
/// @notice Contains all functions which can only be called by privileged users.
interface ICatalystV1PoolAdministration {
    function setFeeAdministrator(address administrator) external;

    function setPoolFee(uint256 fee) external;

    function setGovernanceFee(uint256 fee) external;

    /**
     * @notice Initializes the pool pricing parameters.
     * @param assets The list of assets the pool will support.
     * @param weights The weights of the tokens.
     * @param amp Pool amplification.
     * @param depositor The account to which the initial pool tokens are minted to.
     */
    function initializeSwapCurves(
        address[] calldata assets,
        uint256[] calldata weights,
        uint256 amp,
        address depositor
    ) external;

    /**
     * @notice Creates a connection to the pool _poolReceiving on the channel _channelId.
     * @dev if _poolReceiving is an EVM pool, it can be computes as:
     *     Vyper: convert(_poolAddress, bytes32)
     *     Solidity: abi.encode(_poolAddress)
     *     Brownie: brownie.convert.to_bytes(_poolAddress, type_str="bytes32")
     * setupMaster == ZERO_ADDRESS
     * @param channelId The _channelId of the target pool.
     * @param toPool The bytes32 representation of the target pool
     * @param state Boolean indicating if the connection should be open or closed.
     */
    function setConnection(
        bytes32 channelId,
        bytes calldata toPool,
        bool state
    ) external;

    /**
     * @notice Gives up short term ownership of the pool. This makes the pool unstoppable.
     */
    function finishSetup() external;
}
