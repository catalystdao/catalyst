//SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.16;

import "./interfaces/ICatalystV1VaultSuccessFailure.sol";
import "./interfaces/ICatalystV1VaultAdministration.sol";
import "./interfaces/ICatalystV1VaultDerived.sol";
import "./interfaces/ICatalystV1VaultErrors.sol";
import "./interfaces/ICatalystV1VaultEvents.sol";
import "./interfaces/ICatalystV1VaultImmutables.sol";
import "./interfaces/ICatalystV1VaultPermissionless.sol";
import "./interfaces/ICatalystV1VaultState.sol";
import "./interfaces/ICatalystV1Underwriting.sol";

interface ICatalystV1Vault is
    ICatalystV1VaultSuccessFailure,
    ICatalystV1VaultAdministration,
    ICatalystV1VaultDerived,
    ICatalystV1VaultEvents,
    ICatalystV1VaultImmutables,
    ICatalystV1VaultPermissionless,
    ICatalystV1VaultState,
    ICatalystV1Underwriting
{}
