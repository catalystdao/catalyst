// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import {Token} from "../../mocks/token.sol";
import { WETH9 } from "../../mocks/weth9.sol";

import { CatalystRouter } from "src/router/CatalystRouter.sol";
import { RouterParameters } from "src/router/base/RouterImmutables.sol";

/// @title Interface for WETH9
interface IWETH9 {
    /// @notice Deposit ether to get wrapped ether
    function deposit() external payable;

    /// @notice Withdraw wrapped ether to get ether
    function withdraw(uint256) external;
}

contract TestRouterUnwrapProfile is Test {
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
    
    function test_profile_unwrap() external {
        uint256 amount = uint256(0x11111111111111111);

        weth9.deposit{value: amount}();

        bytes memory commands = abi.encodePacked(bytes1(0x1f), bytes1(0x09));
        bytes memory transfer_from = abi.encode(
            address(weth9),
            address(router),  // This is more expensive than using map but is better way to estimate costs.
            amount
        );
        bytes memory unwarp_gas = abi.encode(
            address(router),
            amount
        );

        bytes[] memory inputs = new bytes[](2);
        inputs[0] = transfer_from;
        inputs[1] = unwarp_gas;

        Token(address(weth9)).approve(address(router), amount + 1);

        router.execute{value: amount}(commands, inputs);
    }
}