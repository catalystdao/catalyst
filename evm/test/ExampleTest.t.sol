// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import { Token } from "./mocks/token.sol";
import { TestCommon } from "./TestCommon.t.sol";
import { ICatalystV1Vault, ICatalystV1Structs } from "../src/ICatalystV1Vault.sol";

contract ExampleTest is TestCommon {

  address vault1;
  address vault2;

  bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0xdead)));

  function setUp() public override {
    // Calls setup() on testCommon
    super.setUp();

    // Create relevant arrays for the vault.
    uint256 numTokens = 2;
    address[] memory assets = new address[](numTokens);
    uint256[] memory init_balances = new uint256[](numTokens);
    uint256[] memory weights = new uint256[](numTokens);

    // Deploy a token
    assets[0] = address(new Token("TEST", "TEST", 18, 1e6));
    init_balances[0] = 1000 * 1e18;
    weights[0] = 1;
    // Deploy another token
    assets[1] = address(new Token("TEST2", "TEST2", 18, 1e6));
    init_balances[1] = 1000 * 1e18;
    weights[1] = 1;

    // Set approvals.
    Token(assets[0]).approve(address(catFactory), init_balances[0] * 2);
    Token(assets[1]).approve(address(catFactory), init_balances[1] * 2);

    vault1 = catFactory.deployVault(
      address(volatileTemplate), assets, init_balances, weights, 10**18, 0, "Example Pool1", "EXMP1", address(CCI)
    );
    vault2 = catFactory.deployVault(
      address(volatileTemplate), assets, init_balances, weights, 10**18, 0, "Example Pool2", "EXMP2", address(CCI)
    );
  }

  function test_localswap() external {
    // Make an account for testing
    address alice = makeAddr("Alice");
    uint256 swapAmount = 100 * 10**18;

    // Get the token at index 0 from the vault
    address fromToken = ICatalystV1Vault(vault1)._tokenIndexing(0);
    // Lets also get the to token while we are at it:
    address toToken = ICatalystV1Vault(vault1)._tokenIndexing(1);

    Token(fromToken).transfer(alice, swapAmount);

    // Approve as alice.
    vm.prank(alice);
    Token(fromToken).approve(vault1, swapAmount);

    uint256 minOut = 0;
    vm.prank(alice);
    uint256 swapReturn = ICatalystV1Vault(vault1).localSwap(fromToken, toToken, swapAmount, minOut);
    
    assertEq(swapReturn, Token(toToken).balanceOf(alice), "Alice didn't get enough tokens");
  }

  function test_cross_chain_swap() external {
    // We need to set address(CCI) as the allowed caller and address(GARP) as the destination.
    bytes memory approvedRemoteCaller = convertEVMTo65(address(CCI));
    bytes memory remoteGARPImplementation = abi.encode(address(GARP));
    // notice that remoteGARPImplementation needs to be encoded with how the AMB expectes it
    // and approvedRemoteCaller needs to be encoded with how GARP expects it.
    CCI.connectNewChain(DESTINATION_IDENTIFIER, approvedRemoteCaller, remoteGARPImplementation);

    ICatalystV1Vault(vault1).setConnection(
      DESTINATION_IDENTIFIER,
      convertEVMTo65(vault2),
      true
    );

    ICatalystV1Vault(vault2).setConnection(
      DESTINATION_IDENTIFIER,
      convertEVMTo65(vault1),
      true
    );

    // Get the token at index 0 from the vault
    address fromToken = ICatalystV1Vault(vault1)._tokenIndexing(0);
    // Lets also get the to token while we are at it:
    address toToken = ICatalystV1Vault(vault1)._tokenIndexing(1);

    // Make an account for testing
    address alice = makeAddr("Alice");
    uint256 swapAmount = 100 * 10**18;

    payable(alice).transfer(_getTotalIncentive(_INCENTIVE));
    Token(fromToken).transfer(alice, swapAmount);
    vm.prank(alice);
    Token(fromToken).approve(vault1, swapAmount);

    // Define the route as a struct:
    ICatalystV1Structs.RouteDescription memory routeDescription = ICatalystV1Structs.RouteDescription({
        chainIdentifier: DESTINATION_IDENTIFIER,
        toVault: convertEVMTo65(vault2),
        toAccount: convertEVMTo65(alice),
        incentive: _INCENTIVE,
        deadline: uint64(0)
    });

    // We need the log emitted by the mock Generalised Incentives implementation.
    vm.recordLogs();
    vm.prank(alice);
    ICatalystV1Vault(vault1).sendAsset{value: _getTotalIncentive(_INCENTIVE)}(
        routeDescription,
        fromToken,
        1,
        swapAmount,
        0,
        alice,
        0,
        hex""
    );
    // Get logs.
    Vm.Log[] memory entries = vm.getRecordedLogs();
    // Decode log.
    (, , bytes memory messageWithContext) = abi.decode(entries[1].data, (bytes32, bytes, bytes));
    // Get GARP message.
    (bytes memory _metadata, bytes memory toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);
    // Process message / Execute the receiveAsset call. This delivers the assets to the user.
    vm.recordLogs();
    GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPITANT);
    // We need to deliver the ack, so we need to relay another message back:
    entries = vm.getRecordedLogs();
    (, , messageWithContext) = abi.decode(entries[3].data, (bytes32, bytes, bytes));
    (_metadata, toExecuteMessage) = getVerifiedMessage(address(GARP), messageWithContext);
    // Process ack
    vm.recordLogs();
    GARP.processPacket(_metadata, toExecuteMessage, FEE_RECIPITANT);

    assertGt(Token(toToken).balanceOf(alice), 0, "Alice didn't get any tokens");
  }
}
