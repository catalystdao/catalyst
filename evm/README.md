# The EVM implementation

The EVM implementation of Catalyst is in Solidity. It serves as a reference implementation while also implementing common optimisations to be efficient and performant. It also defines the message structure that other implementations should honor.

The general structure of a Catalyst implementation is based around Vaults:

- **Vaults** holds assets and the logic for converting tokens into Units.
  - Vaults can be connected together to form a pool. Within a pool, all assets can be swapped for each other.
- **Factory** simplifies the deployment of new vaults.
- **Cross-chain interface** converts swap context into a message which can be sent cross-chain.

More specifically, the code structure is as follows:

- `CatalystVaultCommon.sol` : Defines the structure of a Catalyst vault and implements logic that is common to all vaults.
  - `CatalystVaultVolatile.sol` : Extends `CatalystVaultCommon.sol` with the price curve $P(w) = \frac{W}{w}$.
  - `CatalystVaultAmplified.sol` : Extends `CatalystVaultCommon.sol` with the price curve $P(w) = \left(1 - \theta\right) \frac{W}{(W w)^\theta}$.
  - `FixedPointMathLib.sol` : The mathematical library used by Catalyst (based on the [solmate](https://github.com/transmissions11/solmate/blob/ed67feda67b24fdeff8ad1032360f0ee6047ba0a/src/utils/FixedPointMathLib.sol)).
- `CatalystFactory.sol` : Simplifies the deployment of vaults via Open Zeppelin's *Clones*: vaults are deployed as minimal proxies which delegate call to the above vault contracts. This significantly reduces vault deployment cost.
- `CatalystChainInterface.sol` : Bridges the Catalyst protocol with [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives) which enables Catalyst to support any AMB through the same interface and with the same impact on user experience.

# Catalyst Contracts

## CatalystVaultCommon.sol

An `abstract` contract (i.e. a contract that is made to be overriden), which enforces the core structure of a Catalyst vault and implements features which are generic to any pricing curve. Among these are:

- Vault administration, including fees and vault connections management
- Cross chain swaps acknowledgement and timeout
- Security limit

Note that contracts derived from this one cannot be used directly but should be used via proxy contracts. For this, `CatalystVaultCommon.sol` implements [Initializable.sol](https://docs.openzeppelin.com/contracts/4.x/api/proxy#Initializable) to ensure that vault proxies are correctly setup.

## CatalystVaultVolatile.sol

Extends `CatalystVaultCommon.sol` with the price curve $P(w) = \frac{W}{w}$. This implements the constant product AMM (also called $x \cdot y = k$), known from Uniswap v2 and Balancer.

## CatalystVaultAmplified.sol

Extends `CatalystVaultCommon.sol` with the price curve $P(w) = \left(1 - \theta\right) \frac{W}{(W w)^\theta}$. This introduces an argument $\theta$ which gives control over the flatness of the swap curve, such that the marginal price between assets is closer to 1:1 for a greater amount of swaps. With $\theta = 0$ the pool always delivers 1:1 swaps. This resembles Stable Swap, but with the advantage of allowing for asynchronous swaps.

## CatalystFactory.sol

`CatalystFactory.sol` handles the deployment and configuration of Catalyst vaults proxy contracts within a single call.

## CatalystChainInterface.sol

An intermediate contract designed to interface Catalyst vaults with [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives) AMB interfaces. It wraps and unwraps swap calls to and from byte arrays. Furthermore, it also allows swaps to be underwritten where an external actor takes on the confirmation risk of the swap.

Catalyst v1 implements 2 type of swaps, *Asset Swaps* and *Liquidity Swaps*. The byte array specification for these can be found in `/contracts/CatalystPayload.sol`.

- <u>`0x00`: Asset Swap</u><br/> Swaps with context `0x00` define asset swaps. Although primarily designed for cross-chain asset swaps, there is nothing from stopping a user of *Asset Swapping* between 2 vaults on the same chain.
- <u>`0x01`: Liquidity Swap</u><br/> Swaps with context `0x01` define liquidity swaps. These reduce the cost of rebalancing the liquidity distribution across vaults by combining the following steps into a single transaction:
  1. Withdraw tokens
  2. Convert tokens to units and transfer to target vault
  3. Convert units to an even mix of tokens
  4. Deposit the tokens into the vault.

# Development with Foundry

This repository uses Foundry for testing and development.

## Dev dependencies

- Install `foundryup`
  
  - https://book.getfoundry.sh/getting-started/installation

## Contracts

All contracts are stored in *`./contracts`*. These can be compiled by Foundry with `forge compile`, which will save the compilation output to *`./out`*. Foundry will automatically download the required solidity version.

## Running tests

Catalyst tests can be found within `./test`. A dedicated readme exists within which describes how tests are organised.

To run the tests with Foundry:

```
forge test -vvv
```

Compiling the tests takes a significant amount of time but running the tests themselves is almost instant. The `-vvv` argument prints trace for any failling tests. Many tests are designed for fuzzing. By default, 100 fuzzes are made. To increase the number of runs add the argument `--fuzz-runs 1000`. If the number of runs is particularly high (>10000) some tests might fail with "rejected too many inputs".

## Coverage

Coverage currently doesn't work. It is unclear if this is an issue with Foundry or Solidity. The repository uses the Soldiity pipeline `--via-ir` to circumvent the *stack too deep* issue. The result is that when Foundry tries to re-compile the contracts without any optimisations it fails.

The forge argument `-ir-minimum` has to be used to compile the contracts using the `ir` representation. Note that this changes the mapping of source code to compiled code and some sections can be incorrectled marked as uncovered or covered.

```
forge coverage --ir-minimum
```

Currently, this doesn't work.

# Deploying Catalyst

This repository contains a helper script for deployment `script/DeployCatalyst.s.sol` which is based on `script/DeployContracts.s.sol` which is the origin for most of the testing configuration. This deploys core swap contracts but not the cross-chain interface. This is instead done by `script/DeployInterfaces.s.sol` which also handles management/deployment of the dependency on [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives).

## Local Catalyst

Local Catalyst consists of Volatile and Amplified pools along with the Factory. To deploy Local Catalyst to another chain, add the chain config to `script/BaseMultiChainDeployer.s.sol`. For chains without EIP-1559 add them as a legacy chain.Then run `forge script DeployCatalyst --sig "deploy()" --broadcast` or `forge script DeployCatalyst --sig "deploy_legacy()" --legacy --broadcast` depending on if the chain added was with EIP-1559 support (non-legacy) or with (legacy). Some chains require running with `--slow`. If deployment fails, wait a few blocks and re-try.

## Cross-chain Catalyst

Cross-chain Catalyst requires governance approval. This is unavoidable, since there are no trustless way to verify which chain identifier belongs to which chain. While the cross-chain interface can be deployed by anyone, the setup can only be done by the pre-designated address.

## Deployment verification

The deployment scheme is designed such that any deployment which matches the addresses in `script/config/config_contracts.json` is legitimate. This makes it easy for anyone to deploy, verify, and scale Catalyst.

## Catalyst Setup

To easily interact with Catalyst, you can create a script. Start by importing `script/deployCatalyst.s.sol`. This script allows you to easily deploy the core protocol along with any dependencies.

Below, we will go over writing a test which does a localswap, cross-chain swap, and underwrites a swap. The resulting file generated by this tutorial can be found in `test/ExampleTest.t.sol`

Start by declaring the scaffolding:

```solidity
// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import { TestCommon } from "./TestCommon.t.sol";

contract ExampleTest is TestCommon {
    ...
}
```

By importing `TestCommon` Catalyst is already deployed. It also has several helpers which can simply development of tests.

Catalyst is deployed through a `setUp()` function which is called before any test is executed. Since we want to implement multiple functions, we want to add further logic to the contract initiation. Lets deploy a vault:

```solidity
contract ExampleTest is TestCommon {
  address vault1;
  address vault2;
  function setUp() public override {
    // Calls setup() on testCommon
    super.setup();

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
}
```

We have deployed 2 vaults with the same tokens so we can do a cross-chain swap between the 2 of them later and also compare the output of localswap vs cross-chain swap.


### Execute a LocalSwap

Lets execute a localswap. That is a swap which happens atomically on a single chain to and from the same vault. Before we can do that, we need to allow the vault to take tokens from us. This is done by calling the approve function. For our example, we will be using the token indexed 0 but you can use token index 0, 1 or 2 in this example.

Lets execute a localswap. That is a swap which happens atomically on a single chain to and from the same vault. Before we can do that, we need to allow the vault to take tokens from us. This is done by calling the approve function. For our example, we will be using the token indexed 0 but you can use token index 0, 1 or 2 in this example.

```solidity
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  function setUp() public override {...}

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
  }
}
```

We are now ready to execute a localswpa. Lets swap 50 token0 for token1. A minimum output of 45 tokens is specified (if not fulfilled, the transaction will revert).


```solidity
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  function setUp() public override {...}

  function test_localswap() external {
    ...
    // Approve as alice.
    vm.prank(alice);
    Token(fromToken).approve(vault1, swapAmount);

    uint256 minOut = 0;
    vm.prank(alice);
    uint256 swapReturn = ICatalystV1Vault(vault1).localSwap(fromToken, toToken, swapAmount, minOut);
    
    assertEq(swapReturn, Token(toToken).balanceOf(alice), "Alice didn't get enough tokens");
  }
}
```

If you want to play around with the minimum output, you can undo the swap `chain.undo()` or continue to execute the same transaction. If you have sufficient allowance, it should happen the second time you execute the swap. Try reducing the minimum output to 40 and executing the swap again.

If you executed more swaps to test the minimum output, please undo those with `chain.undo()`. If you want to test that a cross-chain swap returns exactly the same amount as a localswap, please undo the localswap by another `chain.undo()`.

## Cross-Chain Vault Setup

Before being able of executing a cross-chain swap, we need to setup the associated cross-chain communication. We have already deployed a cross-chain interface but we havn't set it up yet. Lets do that:

```solidity
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  address vault2;

  function setUp() public override {...}

  function test_cross_chain_swap() external {
    // We need to set address(CCI) as the allowed caller and address(GARP) as the destination.
    bytes memory approvedRemoteCaller = convertEVMTo65(address(CCI));
    bytes memory remoteGARPImplementation = abi.encode(address(GARP));
    // notice that remoteGARPImplementation needs to be encoded with how the AMB expectes it
    // and approvedRemoteCaller needs to be encoded with how GARP expects it.
    CCI.connectNewChain(DESTINATION_IDENTIFIER, approvedRemoteCaller, remoteGARPImplementation);
  }
}
```

This flow only has to be done once for each deployment and each chain. Since we are only simulating cross-chain connectivity, we only need to set the connection once. If we were to swap between 2 different chains, we would have to set connect the CCIs on both chains with their respective opposite addresses: (`CCI.connectNewChain(remoteChainIdentifier, remoteCCI, remoteGARP)`).¨
Security for vaults is handled by the vaults and we havn't set that yet.  So lets set a connection for the vaults:

```solidity
import { ICatalystV1Vault } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  address vault2;

  function setUp() public override {...}

  function test_cross_chain_swap() external {
    ...

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
  }
}
```

The encoder `convertEVMTo65` is used. This encodes the address into 64 bytes (for evm this is quite wasteful but it has a purpose) and then prefixes the 64 bytes with a single byte to indicate the address length. For evm this is 20 bytes. This is to standardize *most* addresses to a fixed length. 

## Execute a Cross Chain Swap

The following code swaps 100 token1 to token2 from vault1 to vault2. This is exactly the same as the localswap we executed earlier.

```solidity
import { ICatalystV1Vault, ICatalystV1Structs } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  address vault2;
  bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0)));

  function setUp() public override {...}

  function test_cross_chain_swap() external {
    ...

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
        incentive: _INCENTIVE
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
  }
}
```

The preloaded _INCENTIVE is defined in TestCommon. For more information on cross-chain incentives, see [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives).
The following code swaps 100 token1 to token2 from vault1 to vault2. This is exactly the same as the localswap we executed earlier.

```solidity
import { ICatalystV1Vault, ICatalystV1Structs } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  address vault2;
  bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0)));

  function setUp() public override {...}

  function test_cross_chain_swap() external {
    ...

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
        incentive: _INCENTIVE
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
  }
}
```

The preloaded _INCENTIVE is defined in TestCommon. For more information on cross-chain incentives, see [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives).

The swap has been initiated, but the purchased tokens have not been yet sent to the user. If you run the test now with `-vvvv`, you can observe that there is no transfer back to the user. This is because the cross-chain package has only been emitted but not yet executed. A relayer now has to collect the package and submit it to the target chain.

Lets relay the package. We are using a mock implementation of Generalised Incentives. This allows us to easily produce a valid package by calling `getVerifiedMessage`.

```solidity
import { ICatalystV1Vault, ICatalystV1Structs } from "../src/ICatalystV1Vault.sol";
...
contract ExampleTest is TestCommon {
  address vault1;
  address vault2;
  bytes32 FEE_RECIPITANT = bytes32(uint256(uint160(0)));

  function setUp() public override {...}

  function test_cross_chain_swap() external {
    ...

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
```

If you run both tests: `forge test --match-contract ExampleTest -vvvv` you can see that the result of both swaps are exactly the same.

The final test file can be found in `./test/ExampleTest.t.sol`.

# Other comments

## Slither

*[Slither](https://github.com/crytic/slither) is a Solidity static analysis framework written in Python 3. It runs a suite of vulnerability detectors, prints visual information about contract details, and provides an API to easily write custom analyses. Slither enables developers to find vulnerabilities, enhance their code comprehension, and quickly prototype custom analyses.*

Catalyst has been analyzed using Slither and no major bugs have been found. To rerun the analytics, run:

`slither contracts/<>.sol --solc-args "--base-path . --include-path node_modules --optimize --optimize-runs 9000" --exclude naming-convention`

Alternativly, run `slither contracts` to analyze all contracts.
