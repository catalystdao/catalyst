// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/proxy/Clones.sol";
import "./CatalystIBCInterface.sol";

/**
 * @title Catalyst: Swap Pool Factory
 * @author Catalyst Labs
 */
contract CatalystSwapPoolFactory is Ownable {
    using SafeERC20 for IERC20;

    event PoolDeployed(
        address indexed deployer, // msg.sender
        address indexed pool_address, // The forwarder for the pool template
        address indexed chaininterface, // Which cross chain messaging service is used?
        uint256 k, // amplification
        address[] assets // List of the 3 assets
    );

    // 0: Volatile v1
    // 1: Amplified v1
    mapping(uint256 => address) public _poolTemplate;
    mapping(address => mapping(address => bool)) public IsCreatedByFactory;
    uint256 public _defaultGovernanceFee;

    constructor(
        address volatilePoolTemplate,
        address amplifiedPoolTemplate,
        uint256 initialDefaultGovernanceFee
    ) {
        _poolTemplate[0] = volatilePoolTemplate;
        _poolTemplate[1] = amplifiedPoolTemplate;
        _defaultGovernanceFee = initialDefaultGovernanceFee;
    }

    function setNewDefaultGovernanceFee(uint256 newDefaultGovernanceFee)
        external
        onlyOwner
    {
        require(newDefaultGovernanceFee <= 10**18 / 2); // GovernanceFee is maximum 50%.
        _defaultGovernanceFee = newDefaultGovernanceFee;
    }

    function deploy_swappool(
        uint256 poolTemplateIndex,
        address[] calldata init_assets,
        uint256[] memory init_balances,
        uint256[] calldata weights,
        uint256 amp,
        string memory name,
        string memory symbol,
        address chaininterface
    ) external returns (address) {
        address swapPool = Clones.clone(_poolTemplate[poolTemplateIndex]);

        // The pool expects the balances to exist in the pool when setup is called.
        for (uint256 it = 0; it < init_assets.length; it++) {
            IERC20(init_assets[it]).safeTransferFrom(
                msg.sender,
                swapPool,
                init_balances[it]
            );
        }

        ICatalystV1Pool(swapPool).setup(
            init_assets,
            weights,
            amp,
            _defaultGovernanceFee,
            name,
            symbol,
            chaininterface,
            msg.sender
        );

        emit PoolDeployed(
            msg.sender,
            swapPool,
            chaininterface,
            amp,
            init_assets
        );
        IsCreatedByFactory[chaininterface][swapPool] = true;

        return swapPool;
    }
}
