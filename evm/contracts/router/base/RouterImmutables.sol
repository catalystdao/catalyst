// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

import {ERC20} from 'solmate/src/tokens/ERC20.sol';
import {IWETH9} from '../interfaces/external/IWETH9.sol';

struct RouterParameters {
    address weth9;
}

/// @title Router Immutable Storage contract
/// @notice Used along with the `RouterParameters` struct for ease of cross-chain deployment
contract RouterImmutables {
    /// @dev WETH9 address
    IWETH9 internal immutable WETH9;

    constructor(RouterParameters memory params) {
        WETH9 = IWETH9(params.weth9);
    }
}