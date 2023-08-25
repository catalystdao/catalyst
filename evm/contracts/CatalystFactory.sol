// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.16;

import "@openzeppelin/contracts/access/Ownable.sol";
import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {SafeTransferLib} from 'solmate/src/utils/SafeTransferLib.sol';
import "@openzeppelin/contracts/proxy/Clones.sol";
import "./CatalystIBCInterface.sol";
import "./interfaces/ICatalystV1FactoryEvents.sol";

uint256 constant MAX_GOVERNANCE_FEE_SHARE = 75e16;   // 75%

/**
 * @title Catalyst Swap Factory
 * @author Cata Labs
 * @notice Allows permissionless deployment Catalyst vaults
 * and defines governance address for vaults to read.
 * !The owner of the factory must be a timelock!
 */
contract CatalystFactory is Ownable, ICatalystV1FactoryEvents {
    using SafeTransferLib for ERC20;

    /// @notice A mapping which describes if a vault has been created by this factory. Indexed by chainInterface then vault address.
    mapping(address => mapping(address => bool)) public isCreatedByFactory;

    /// @notice Default governance fee. When a vault is created, this is the governance fee applied to that vault.
    uint256 public _defaultGovernanceFee;

    constructor(uint256 defaultGovernanceFee) {
        setDefaultGovernanceFee(defaultGovernanceFee);
    }

    function setDefaultGovernanceFee(uint256 fee) public onlyOwner {
        require(fee <= MAX_GOVERNANCE_FEE_SHARE); // dev: Maximum GovernanceFeeSare exceeded.

        emit SetDefaultGovernanceFee(fee);

        _defaultGovernanceFee = fee;
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
        uint256 amp,
        uint256 vaultFee,
        string calldata name,
        string calldata symbol,
        address chainInterface
    ) external returns (address) {
        // Check if an invalid asset count has been provided
        require(assets.length > 0);  // dev: invalid asset count
        // Check if an invalid weight count has been provided
        require(weights.length == assets.length); //dev: invalid weight count
        // init_balances length not checked: if shorter than assets, the funds transfer loop
        // will fail. If longer, values will just be ignored.

        // Create a minimal transparent proxy:
        address vault = Clones.clone(vaultTemplate);

        // The vault expects the balances to exist in the vault when setup is called.
        for (uint256 it; it < assets.length; it++) {
            ERC20(assets[it]).safeTransferFrom(
                msg.sender,
                vault,
                init_balances[it]
            );
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