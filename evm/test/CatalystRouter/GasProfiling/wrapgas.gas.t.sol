// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import {Token} from "../../mocks/token.sol";
import { WETH9 } from "../../mocks/weth9.sol";

import { CatalystRouter } from "src/router/CatalystRouter.sol";
import { RouterParameters } from "src/router/base/RouterImmutables.sol";

contract TestRouterWrapProfile is Test {
    address internal constant ADDRESS_THIS = address(2);

    address TO_ACCOUNT = address(uint160(0xe00acc084f));

    address _REFUND_GAS_TO = TO_ACCOUNT;

    CatalystRouter router;

    WETH9 weth9;

    function setUp() public virtual {
        weth9 = new WETH9();

        router = new CatalystRouter(
            RouterParameters({permit2: address(0), weth9: address(weth9)})
        );
    }
    
    function test_profile_wrap() external {
        uint256 amount = uint256(0x11111111111111111);

        bytes memory commands = abi.encodePacked(bytes1(0x08));
        bytes memory warp_gas = abi.encode(
            address(this),
            amount
        );

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = warp_gas;

        router.execute{value: amount}(commands, inputs);
    }
}