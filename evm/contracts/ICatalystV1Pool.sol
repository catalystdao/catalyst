//SPDX-License-Identifier: Unlicsened
pragma solidity >=0.8.17 <0.9.0;

import "./interfaces/ICatalystV1PoolAckTimeout.sol";
import "./interfaces/ICatalystV1PoolAdministration.sol";
import "./interfaces/ICatalystV1PoolDerived.sol";
import "./interfaces/ICatalystV1PoolErrors.sol";
import "./interfaces/ICatalystV1PoolEvents.sol";
import "./interfaces/ICatalystV1PoolImmutables.sol";
import "./interfaces/ICatalystV1PoolPermissionless.sol";
import "./interfaces/ICatalystV1PoolState.sol";

interface ICatalystV1Pool is
    ICatalystV1PoolAckTimeout,
    ICatalystV1PoolAdministration,
    ICatalystV1PoolDerived,
    ICatalystV1PoolEvents,
    ICatalystV1PoolImmutables,
    ICatalystV1PoolPermissionless,
    ICatalystV1PoolState
{}
