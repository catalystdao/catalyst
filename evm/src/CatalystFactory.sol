// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.19;

import { Ownable } from "solady/auth/Ownable.sol";
import { SafeTransferLib } from 'solady/utils/SafeTransferLib.sol';
import { LibClone } from "solady/utils/LibClone.sol";

import { ICatalystV1Factory } from "./interfaces/ICatalystV1Factory.sol";
import { ICatalystV1Vault } from "./ICatalystV1Vault.sol";

uint256 constant MAX_GOVERNANCE_FEE_SHARE = 75e16;   // 75%

/**
 * @title Catalyst Swap Factory
 * @author Cata Labs
 * @notice Allows permissionless deployment Catalyst vaults
 * and defines governance address for vaults to read.
 * !The owner of the factory must be a timelock!
 */
contract CatalystFactory is Ownable, ICatalystV1Factory {

    /// @notice A mapping which describes if a vault has been created by this factory. Indexed by chainInterface then vault address.
    mapping(address => mapping(address => bool)) public isCreatedByFactory;

    /// @notice Default governance fee. When a vault is created, this is the governance fee applied to that vault.
    uint64 public _defaultGovernanceFee;

    address public _governanceFeeDestination;

    constructor(address defaultOwner) payable {
        _initializeOwner(defaultOwner);
        _governanceFeeDestination = defaultOwner;
    }

    function setDefaultGovernanceFee(uint64 fee) override public onlyOwner {
        require(fee <= MAX_GOVERNANCE_FEE_SHARE); // dev: Maximum GovernanceFeeSare exceeded.

        emit SetDefaultGovernanceFee(fee);

        _defaultGovernanceFee = fee;
    }

    function setGovernanceFeeDestination(address feeDestination) override public onlyOwner {
        require(feeDestination != address(0), "Fee destination cannot be address(0)");
        emit SetGovernanceFeeDestination(feeDestination);

        _governanceFeeDestination = feeDestination;
    }

    /**
     * @notice Deploys a Catalyst vault, funds the vault with tokens, and calls setup.
     * @dev The deployer needs to set approvals for this contract before calling deployVault
     * @param vaultTemplate The template the transparent proxy should target.
     * @param assets The list of assets the vault should support.
     * @param init_balances The initial balances of the vault. (Should be approved)
     * @param weights The weights of the tokens.
     * @param amp Token parameter 1. (Amplification)
     * @param vaultFee The vault fee.
     * @param name Name of the Vault token.
     * @param symbol Symbol for the Vault token.
     * @param chainInterface The cross chain interface used for cross-chain swaps. (Can be address(0) to disable cross-chain swaps.)
     * @return address The address of the created Catalyst Vault. (minimal transparent proxy)
     */
    function deployVault(
        address vaultTemplate,
        address[] calldata assets,
        uint256[] calldata init_balances,
        uint256[] calldata weights,
        uint64 amp,
        uint64 vaultFee,
        string memory name,
        string memory symbol,
        address chainInterface
    ) override external returns (address) {
        // Check if an invalid asset count has been provided
        require(assets.length != 0);  // dev: invalid asset count
        // Check if an invalid weight count has been provided
        require(weights.length == assets.length); //dev: invalid weight count
        // init_balances length not checked: if shorter than assets, the funds transfer loop
        // will fail. If longer, values will just be ignored.

        // Create a minimal transparent proxy:
        address vault = LibClone.clone(vaultTemplate);

        // The vault expects the balances to exist in the vault when setup is called.
        uint256 assetLength = assets.length;
        for (uint256 it; it < assetLength;) {
            SafeTransferLib.safeTransferFrom(
                assets[it],
                msg.sender,
                vault,
                init_balances[it]
            );

            unchecked {
                ++it;
            }
        }

        // Call setup
        ICatalystV1Vault(vault).setup(
            name,
            symbol,
            chainInterface,
            vaultFee,
            _defaultGovernanceFee,
            owner(),        // Fee administrator
            msg.sender      // setup master
        );

        // Initialize swap curves
        ICatalystV1Vault(vault).initializeSwapCurves(
            assets,
            weights,
            amp,
            msg.sender
        );

        // Emit event for vault discovery.
        emit VaultDeployed(
            vaultTemplate,
            chainInterface,
            msg.sender,
            vault,
            assets,
            amp
        );
        isCreatedByFactory[chainInterface][vault] = true;

        return vault;
    }
}
