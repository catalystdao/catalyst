// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import {Token} from "../../mocks/token.sol";

import { CatalystRouter } from "src/router/CatalystRouter.sol";
import { RouterParameters } from "src/router/base/RouterImmutables.sol";

contract TestRouterTransferFromProfile is Test {
    address internal constant ADDRESS_THIS = address(2);

    // add this to be excluded from coverage report
    function test() public {}

    CatalystRouter router;

    function setUp() public {
        router = new CatalystRouter(
            RouterParameters({permit2: address(0), weth9: address(0x4200000000000000000000000000000000000006)})
        );
    }
    
    function test_profile_transfer_from() external {
        address token1 = address(new Token("hello World", "HELWOD", 18, 1e6));

        uint256 amount = uint256(0x11111111111111111);

        Token(token1).approve(address(router), amount + 1);

        bytes memory commands = abi.encodePacked(bytes1(0x1f));

        bytes memory transfer_from = abi.encode(
            token1,
            address(router),  // This is more expensive than using map but is better way to estimate costs.
            amount
        );

        bytes[] memory inputs = new bytes[](1);
        inputs[0] = transfer_from;

        router.execute(commands, inputs);
        
    }
}