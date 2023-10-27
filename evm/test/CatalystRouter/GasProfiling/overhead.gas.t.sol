// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import {Token} from "../../mocks/token.sol";

import { CatalystRouter } from "src/router/CatalystRouter.sol";
import { RouterParameters } from "src/router/base/RouterImmutables.sol";

contract TestRouterOverheadProfile is Test {
    address internal constant ADDRESS_THIS = address(2);

    // add this to be excluded from coverage report
    function test() public {}

    CatalystRouter router;

    function setUp() public {
        router = new CatalystRouter(
            RouterParameters({permit2: address(0), weth9: address(0x4200000000000000000000000000000000000006)})
        );
    }
    
    function test_profile_router_overhead() external {
        bytes memory commands = abi.encodePacked();

        bytes[] memory inputs = new bytes[](0);

        router.execute(commands, inputs);
        
    }
}