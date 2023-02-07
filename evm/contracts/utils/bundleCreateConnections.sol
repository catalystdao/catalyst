//SPDX-License-Identifier: MIT

pragma solidity ^0.8.16;

interface ISwapPool {
    function setConnection(
        uint256 _chain,
        bytes32 _poolReceiving,
        bool _state
    ) external returns (bool);

    function finishSetup() external returns (bool);
}

/// @title bundleConnections
/// @author Alexander @ Polymer Labs
/// @notice
///     Improves the UI for Catalyst by only requiring the user to create
///     the pool connections + finishSetup() one time for each chain.
contract bundleConnections {
    struct connection {
        uint256 _chain;
        bytes32 _poolReceiving;
    }

    /// @notice Allows an array of connections to passed a newly setup pool and finishs setup.
    /// @dev All connections are set to true. Uses the tx.origin check.
    /// @param _pool address; The pool to setup.
    /// @param _connectionsBundle connection[]; An array of the struct connection.
    function connectBundle(
        address _pool,
        connection[] memory _connectionsBundle
    ) external {
        for (uint256 i = 0; i < _connectionsBundle.length; i++) {
            connection memory connExtract = _connectionsBundle[i];
            require(
                ISwapPool(_pool).setConnection(
                    connExtract._chain,
                    connExtract._poolReceiving,
                    true
                ),
                "Connection creation failed"
            );
        }
        require(ISwapPool(_pool).finishSetup(), "Could not finish setup");
    }
}
