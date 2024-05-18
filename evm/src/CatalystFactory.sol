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
 * @author Cata Labs Inc.
 * @notice Allows permissionless deployment Catalyst vaults and defines governance address for vaults to read.
 * Importantly, this vault allows anyone to deploy a vault using any vault template and vault cross-chain interface.
 * As a result, just because a vault was deployed using this contract does not imply that it is safe. Vaults should
 * be cross-checked for their template, cross-chain interface, and if they are setup correctly. It may even be
 * that some vault templates only work with some cross-chain interfaces.
 *
 * Using the reference Catalyst Vault Templates, the owner of the factory is also the _owner_ of the Vaults.
 * They have certain privilege that may be able to be abused depending on the vault. One of these is configurating
 * fees. As a result:
 * !The owner of the factory must be a timelock!
 */
contract CatalystFactory is Ownable, ICatalystV1Factory {

    error InvalidAssetCount();
    error InvalidWeightCount();
    error FeeDestinationAddress0();
    error MaximumGovernanceFeeShare();

    /**
     * @notice A mapping which describes if a vault has been created by this factory.
     * Indexed by chainInterface then vault address.
     */
    mapping(address => mapping(address => bool)) public isCreatedByFactory;

    /** 
     * @notice Default governance fee. When a vault is created, this is the governance fee applied to that vault.
     */
    uint64 public _defaultGovernanceFee;

    /**
     * @notice The address to send governance fees to.
     * @dev Not enforced by the factory but vaults are expected to follow it.
     */
    address public _governanceFeeDestination;

    // The 2 above storage slots are packed together.

    constructor(address defaultOwner) payable {
        _initializeOwner(defaultOwner);
        _governanceFeeDestination = defaultOwner;
    }

    /**
     * @notice Set default governance fee share. 
     * @dev The set governance fee only applies to newly created vaults. Vaults have to be individual modified post creation. 
     * Is in WAD, (1e18 terms). So 1e16 is 1%. Cannot be set larger than 75% (75e16).
     */
    function setDefaultGovernanceFee(uint64 fee) override public onlyOwner {
        if (fee > MAX_GOVERNANCE_FEE_SHARE) revert MaximumGovernanceFeeShare();

        emit SetDefaultGovernanceFee(fee);

        _defaultGovernanceFee = fee;
    }

    /**
     * @notice Set the recipient of the governance.
     * @dev It is expected that vaults read this value and send their governance fees here.
     * This contract has no way to enforce if vaults honour this value.
     * Cannot be set to address(0). If wish to burn, set to 0xdead.
     */
    function setGovernanceFeeDestination(address feeDestination) override public onlyOwner {
        if (feeDestination == address(0)) revert FeeDestinationAddress0();
        emit SetGovernanceFeeDestination(feeDestination);

        _governanceFeeDestination = feeDestination;
    }

    /**
     * @notice Deploys a Catalyst vault, funds the vault with tokens, and calls setup.
     * When deploying vaults, there are 2 stages that needs to happen:
     * 1. We need to setup the vaults with the correct configuration.
     * 2. We need to set the vault swap curve. This consists of setting assets, vaults, amplification, etc.
     * The reason it is done in 2 steps is because of the stack limit. By spreading it across 2 calls, it is
     * cheaper gas wise.
     * This is done in a safe way by expecting both of these init. calls to be done in a single transaction.
     * As a result, the vaults are never left in a vulnerable state. It is expected that the latter call
     * (initializeSwapCurves) completes initialization and blocks the setup functions from being called again.
     * @dev The deployer needs to set relevant approvals to this contract before calling deployVault.
     * @param vaultTemplate The template the transparent proxy should target.
     * @param assets The list of assets the vault should support.
     * @param init_balances The initial balances of the vault. (Should be approved)
     * @param weights The weights of the tokens.
     * @param amp Token parameter 1. (Amplification)
     * @param vaultFee The vault fee.
     * @param name Name of the Vault token.
     * @param symbol Symbol for the Vault token.
     * @param chainInterface The cross chain interface used for cross-chain swaps. (Can be address(0) to disable cross-chain swaps.)
     * @return vault The address of the created Catalyst Vault. (minimal transparent proxy)
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
    ) override external returns (address vault) {
        // Check if an invalid asset count has been provided
        if (assets.length == 0) revert InvalidAssetCount();
        // Check if an invalid weight count has been provided
        if (weights.length != assets.length) revert InvalidWeightCount();
        // init_balances length not checked: if shorter than assets, the funds transfer loop
        // will fail. If longer, values will just be ignored.

        // Create a minimal transparent proxy:
        vault = LibClone.clone(vaultTemplate);

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
