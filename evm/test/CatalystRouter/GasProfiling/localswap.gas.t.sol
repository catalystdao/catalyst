// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "src/ICatalystV1Vault.sol";
import { TestCommon } from "../../TestCommon.t.sol";
import {Token} from "../../mocks/token.sol";

import { CatalystRouter } from "src/router/CatalystRouter.sol";
import { RouterParameters } from "src/router/base/RouterImmutables.sol";

contract TestRouterLocalswapProfile is TestCommon {
    address internal constant ADDRESS_THIS = address(2);

    address TO_ACCOUNT = address(uint160(0xe00acc084f));

    address _REFUND_GAS_TO = TO_ACCOUNT;

    CatalystRouter router;

    function setUp() public virtual override {
        super.setUp();

        router = new CatalystRouter(
            RouterParameters({permit2: address(0), weth9: address(0x4200000000000000000000000000000000000006)})
        );
    }

    function pool1() internal returns(address vault1, address vault2) {
        // Deploy tokens. 
        address[] memory tokens = getTokens(3);
        approveTokens(address(catFactory), tokens);

        // Deploy an amplified vault
        uint256[] memory amounts = new uint256[](3);
        amounts[0] = 100*10**18; amounts[1] = 200*10**18; amounts[2] = 300*10**18;
        uint256[] memory weights = new uint256[](3);
        weights[0] = 1; weights[1] = 1; weights[2] = 1;
        amounts[0] = 100*10**18; amounts[1] = 200*10**18; amounts[2] = 300*10**18;
        weights[0] = 1; weights[1] = 1; weights[2] = 1;
        vault1 = deployVault(
            tokens,
            amounts,
            weights,
            10**18 / 2,
            0
        );

        vault2 = vault1;

        setConnection(vault1, vault2, DESTINATION_IDENTIFIER, DESTINATION_IDENTIFIER);
    }
    
    function test_profile_localswap() external {
        (address fromVault, address toVault) = pool1();
        address fromAsset = ICatalystV1Vault(fromVault)._tokenIndexing(0);
        address toAsset = ICatalystV1Vault(fromVault)._tokenIndexing(1);

        uint256 amount = uint256(0x11111111111111111);

        Token(fromAsset).approve(address(router), amount + 1);

        bytes memory commands = abi.encodePacked(bytes1(0x1f), bytes1(0x00), bytes1(0x04));
        bytes memory transfer_from = abi.encode(
            fromAsset,
            address(router),  // This is more expensive than using map but is better way to estimate costs.
            amount
        );

        bytes memory local_swap = abi.encode(
            fromVault,
            fromAsset,
            toAsset,
            amount,
            uint256(0x111111111111111)
        );

        bytes memory sweep = abi.encode(
            toAsset,
            ADDRESS_THIS,
            uint256(0x111111111111111)
        );

        bytes[] memory inputs = new bytes[](3);
        inputs[0] = transfer_from;
        inputs[1] = local_swap;
        inputs[2] = sweep;

        router.execute(commands, inputs);
        
    }
}