// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.17;

import {IAllowanceTransfer} from '../libraries/permit2/IAllowanceTransfer.sol';
import {ERC20} from 'solmate/tokens/ERC20.sol';
import {IWETH9} from '../interfaces/external/IWETH9.sol';

struct RouterParameters {
    address permit2;
    address weth9;
}

/// @title Router Immutable Storage contract
/// @notice Used along with the `RouterParameters` struct for ease of cross-chain deployment
contract RouterImmutables {
    /// @dev Permit2 address
    IAllowanceTransfer internal immutable PERMIT2;

    /// @dev WETH9 address
    IWETH9 public immutable WETH9;

    constructor(RouterParameters memory params) {
        PERMIT2 = IAllowanceTransfer(params.permit2);
        WETH9 = IWETH9(params.weth9);
    }
}