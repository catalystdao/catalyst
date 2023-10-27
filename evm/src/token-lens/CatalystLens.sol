// SPDX-License-Identifier: MIT

pragma solidity =0.8.19;

import {Multicall2} from "./Multicall2.sol";
import {TokenLens} from "./TokenLens.sol";

contract CatalystLens is Multicall2, TokenLens {}
