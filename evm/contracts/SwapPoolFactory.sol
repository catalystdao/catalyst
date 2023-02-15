// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import "@openzeppelin/contracts/proxy/Clones.sol";
import "./CatalystIBCInterface.sol";
import "./interfaces/ICatalystV1FactoryEvents.sol";

uint256 constant MAX_GOVERNANCE_FEE_SHARE = 75 * 10**16;   // 75%

/**
 * @title Catalyst Swap Factory
 * @author Catalyst Labs
 * @notice Allows permissionless deployment Catalyst Swap pools
 * and defines governance address for swap pools to read.
 * !The owner of the factory must be a timelock!
 */
contract CatalystSwapPoolFactory is Ownable, ICatalystV1FactoryEvents {
    using SafeTransferLib for ERC20;

    mapping(address => mapping(address => bool)) public IsCreatedByFactory;
    uint256 public _defaultGovernanceFee;

    constructor(
        uint256 defaultGovernanceFee
    ) {
        setDefaultGovernanceFee(defaultGovernanceFee);
    }

    function setDefaultGovernanceFee(uint256 fee) public onlyOwner {
        require(fee <= MAX_GOVERNANCE_FEE_SHARE); // dev: Maximum GovernanceFeeSare exceeded.

        emit SetDefaultGovernanceFee(fee);

        _defaultGovernanceFee = fee;
    }

    /**
     * @notice Deploys a Catalyst swap pools, funds the swap pool with tokens, and calls setup.
     * @dev The deployer needs to set approvals for this contract before calling deploy_swappool
     * @param poolTemplate The template the transparent proxy should target.
     * @param assets The list of assets the pool should support
     * @param init_balances The initial balances of the swap pool. (Should be approved)
     * @param weights The weights of the tokens.
     * @param amp Token parameter 1. (Amplification)
     * @param poolFee The pool fee.
     * @param name Name of the Pool token.
     * @param symbol Symbol for the Pool token.
     * @param chainInterface The cross chain interface used for cross-chain swaps. (Can be address(0))
     * @return address The address of the created Catalyst Swap Pool (minimal transparent proxy)
     */
    function deploy_swappool(
        address poolTemplate,
        address[] memory assets,
        uint256[] memory init_balances,
        uint256[] memory weights,
        uint256 amp,
        uint256 poolFee,
        string memory name,
        string memory symbol,
        address chainInterface
    ) external returns (address) {
        // Create a minimal transparent proxy:
        address swapPool = Clones.clone(poolTemplate);

        // The pool expects the balances to exist in the pool when setup is called.
        for (uint256 it = 0; it < assets.length; it++) {
            ERC20(assets[it]).safeTransferFrom(
                msg.sender,
                swapPool,
                init_balances[it]
            );
        }

        // Call setup
        ICatalystV1Pool(swapPool).setup(
            name,
            symbol,
            chainInterface,
            poolFee,
            _defaultGovernanceFee,
            owner(),     // Fee administrator
            msg.sender      // setup master
        );

        // Initialize swap curves
        ICatalystV1Pool(swapPool).initializeSwapCurves(
            assets,
            weights,
            amp,
            msg.sender
        );

        // Emit event for pool discovery.
        emit PoolDeployed(
            msg.sender,
            swapPool,
            chainInterface,
            assets,
            poolTemplate,
            amp
        );
        IsCreatedByFactory[chainInterface][swapPool] = true;

        return swapPool;
    }
}
