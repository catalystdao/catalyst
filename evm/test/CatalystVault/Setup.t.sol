// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "forge-std/Test.sol";
import "../TestCommon.t.sol";
import "../../src/ICatalystV1Vault.sol";
import {Token} from "../mocks/token.sol";
import "../../src/utils/FixedPointMathLib.sol";
import {AVaultInterfaces} from "./AVaultInterfaces.t.sol";
import { ICatalystV1Structs } from "../../src/interfaces/ICatalystV1VaultState.sol";

abstract contract TestSetup is TestCommon, AVaultInterfaces {
    
}
