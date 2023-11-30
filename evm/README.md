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
  - `FixedPointMathLib.sol` : The mathematical library used by Catalyst (based on [solmate](https://github.com/transmissions11/solmate/blob/ed67feda67b24fdeff8ad1032360f0ee6047ba0a/src/utils/FixedPointMathLib.sol)).
- `CatalystFactory.sol` : Simplifies the deployment of vaults via Open Zeppelin's *Clones*: vaults are deployed as minimal proxies which delegate call to the above vault contracts. This significantly reduces vault deployment cost.
- `CatalystChainInterface.sol` : Bridges the Catalyst protocol with [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives) which enables Catalyst to support any AMB through the same interface and with the same great user experience. The CatalystChainInterface also holds logic for underwriting.

# Catalyst Contracts

## CatalystVaultCommon.sol

An `abstract` contract (i.e. a contract that is ment to be overriden), which enforces the core structure of a Catalyst vault and implements features which are generic to any pricing curve. Among these are:

- Vault administration, including fees and vault connections management
- Cross chain swaps acknowledgement and timeout
- Security limit

`CatalystVaultCommon.sol` implements [Initializable.sol](https://docs.openzeppelin.com/contracts/4.x/api/proxy#Initializable) to ensure contract which inherit it are deployed with delegrate proxies rather than using the contract directly.

## CatalystVaultVolatile.sol

Extends `CatalystVaultCommon.sol` with the price curve $P(w) = \frac{W}{w}$. This implements the constant product AMM (also called $x \cdot y = k$), known from Uniswap v2 and Balancer.

## CatalystVaultAmplified.sol

Extends `CatalystVaultCommon.sol` with the price curve $P(w) = \left(1 - \theta\right) \frac{W}{(W w)^\theta}$. This introduces an argument $\theta$ which gives control over the flatness of the swap curve, such that the marginal price between assets is closer to 1:1 for a greater amount of swaps. With $\theta = 0$ the pool always delivers 1:1 swaps. This resembles Stable Swap, but with the advantage of allowing for asynchronous swaps.

## CatalystFactory.sol

`CatalystFactory.sol` handles the deployment and configuration of Catalyst vaults proxy contracts within a single call.

## CatalystChainInterface.sol

An intermediate contract designed to interface Catalyst vaults with [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives) AMB interfaces. It wraps and unwraps swap calls to and from byte arrays. Furthermore, it also allows swaps to be underwritten where an external actor takes on the confirmation risk of the swap.

Catalyst v1 implements 2 type of swaps, *Asset Swaps* and *Liquidity Swaps*. The byte array specification for these can be found in `/contracts/CatalystPayload.sol`.

- `<u>0x00`: Asset Swap `</u><br/>` Swaps with context `0x00` define asset swaps. Although primarily designed for cross-chain asset swaps, there is nothing from stopping a user of *Asset Swapping* between 2 vaults on the same chain. Asset swaps can always be underwritten, it is not possible to opt out of underwriting but it is possible to set the underwriting incentive to 0.
- `<u>0x01`: Liquidity Swap `</u><br/>` Swaps with context `0x01` define liquidity swaps. Liquidity swap cannot be underwritten. Liquidity swaps reduce the cost of rebalancing the liquidity distribution across vaults by combining the following steps into a single transaction:
  1. Withdraw tokens
  2. Convert tokens to units and transfer to target vault
  3. Convert units to an even mix of tokens
  4. Deposit the tokens into the vault.
   

## Dev dependencies

- Install `foundryup`.

  - https://book.getfoundry.sh/getting-started/installation

# Development with Foundry

This repository contains a helper script for deployment `script/DeployCatalyst.s.sol`. This script deploys the core vault contract. It is based on `script/DeployContracts.s.sol` which is used to deploy the test configuration of Catalyst. The core valut contracts include the vault templates and the facotry but not the cross-chain interface. This is instead done by `script/DeployInterfaces.s.sol` which also handles management/deployment of the dependency on [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives).

### Local Catalyst
This repository contains a helper script for deployment `script/DeployCatalyst.s.sol` which is based on `script/DeployContracts.s.sol` which is the origin for most of the testing configuration. The script deploys core swap contracts but not the cross-chain interface. The cross-chain interface is deployed by `script/DeployInterfaces.s.sol` which also handles management/deployment of the dependency on [Generalised Incentives](https://github.com/catalystdao/GeneralisedIncentives).

Local Catalyst consists of Volatile and Amplified pools along with the Factory. To deploy Local Catalyst to another chain, add the chain config to `script/BaseMultiChainDeployer.s.sol`. For chains without EIP-1559 add them as a legacy chain. 
Then run `forge script DeployCatalyst --sig "deploy()" --broadcast` or `forge script DeployCatalyst --sig "deploy_legacy()" --legacy --broadcast` depending on if the chain has EIP-1559 support (non-legacy) or has no support (legacy). Some chains require running with `--slow`. If deployment fails, wait a few blocks and re-try.
This deployment strategy ensures that Catalyst has the same addresses on every chain and it is simple to audit if the contract addresses are correct.

### Cross-chain Catalyst

Cross-chain Catalyst requires governance approval. This is unavoidable, since there are no trustless way to verify which chain identifier belongs to which chain. While the cross-chain interface can be deployed by anyone, the setup can only be done by the pre-designated address.

## Interacting with Catalyst

There are multiple ways to interfact with Catalyst: Creating a test file and importing `test/TestCommon.t.sol` or writing a script and importing `script/DeployCatalyst.s.sol` and `script/DeployInterfaces.s.sol` to access the core protocol along with any dependencies.

Below, we will go over writing a test which does a localswap, cross-chain swap, and underwrites a swap. The resulting file generated by this tutorial can be found in `test/exampleTest.t.sol`

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


## Execute a LocalSwap

Lets execute a localswap. That is a swap which happens atomically on a single chain to and from the same vault. Before we can do that, we need to allow the vault to take tokens from us. This is done by calling the approve function. For our example, we will be using the token indexed 0 but you can use token index 0, 1 or 2 in this example.

```python
tokens[0].approve(vault, 2**256-1, {'from': acct})
```

We are now ready to execute a localswpa. Lets swap 50 token0 for token1. A minimum output of 45 tokens is specified (if not fulfilled, the transaction will revert).

```python
localSwap_tx = vault.localSwap(tokens[0], tokens[1], 50 * 10**18, 45 * 10**18, {'from': acct})
```

If you want to play around with the minimum output, you can undo the swap `chain.undo()` or continue to execute the same transaction. If you have sufficient allowance, it should happen the second time you execute the swap. Try reducing the minimum output to 40 and executing the swap again.

If you executed more swaps to test the minimum output, please undo those with `chain.undo()`. If you want to test that a cross-chain swap returns exactly the same amount as a localswap, please undo the localswap by another `chain.undo()`.

## Cross-Chain Vault Setup

Before being able of executing a cross-chain swap, an IBC channel between vaults must be established. The following establishes a channel to and from the `CatalystIBCInterface.sol` contract, allowing cross-chain swaps between vaults which use this interface.

```python
# Registor IBC ports.
ps.crosschaininterface.registerPort()
ps.crosschaininterface.registerPort()
```

Once the cross-chain interface is properly connected, swaps between the test vault and itself can be allowed. Note that this does not represent a real use case scenario, as vault connections should be created between different vaults and not within the same vault. However, this provides a simple manner in which to test the cross chain capabilities of Catalyst vaults. Lets specific the current channel as 1 and connect the vault to itself:

```python
chid = convert.to_bytes(1, type_str="bytes32")  # Define the channel id to be 1. The emulator ignores this but it is important for the connection.

# Create the connection between the vault and itself:
vault.setConnection(
    chid,
    convert_64_bytes_address(vault.address),
    True,
    {"from": acct}
)
```

Notice that the encoder `convert_64_bytes_address` is used. This encodes the address into 64 bytes (for evm this is quite wasteful but it has a purpose) and then prefixes the 64 bytes with a single byte to indicate the address length. For evm this is 20 bytes. If this is confusing, try the below example:

```python
convert_64_bytes_address(acct.address).hex(), int("14", 16), acct.address
```

The encoded address begins with `14` in hex. This corrosponds to 20 in decimal. Then the last 20 bytes are the same as acct.address.

## Execute a Cross Chain Swap

The following code swaps 50 token0s from token0 to token1 via the cross chain channel defined above. This is exactly the same as the localswap we executed earlier. If you skipped that part, you need to approve the vault to spend token0.

```python
swap_amount = 50 * 10**18
sendAsset_tx = vault.sendAsset(
    chid,
    convert_64_bytes_address(vault.address),  # Set the target vault as itself. (encoded in 64 + 1 bytes)
    convert_64_bytes_address(acct.address),  # Set the target user as acct.   (encoded in 64 + 1 bytes)
    tokens[0],  # Swap out of token0.
    1,  # Swap into token1.
    swap_amount,  # Swap swap_amount of token0.
    40 * 10**18,  # Return more than 40 tokens.
    acct,  # If the transaction reverts, send the tokens back to acct.
    {"from": acct},  # Acct pays for the transactions.
)
```

The swap has been initiated, but the purchased tokens have not been yet sent to the user (this can be observed by inspecting `sendAsset_tx.info()`). This is because the cross-chain package has only been emitted but not yet executed. The relayer now has to collect the package and submit it to the target chain. Before doing this, the IBC payload can be examined to better understand what Catalyst is sending to the target chain:

```python
sendAsset_tx.events["IncomingPacket"]["packet"][3]
decode_payload(sendAsset_tx.events["IncomingPacket"]["packet"][3])
```

Finally, the IBC package can be executed as follows, marking the finalisation of the swap:

```python
swap_execution_tx = ie.execute(sendAsset_tx.events["IncomingMetadata"]["metadata"][0], sendAsset_tx.events["IncomingPacket"]["packet"], {"from": acct})

swap_execution_tx.info()
```

If you ran `chain.undo()` earlier, you can compare the output with the localswap. Notice that the swap outputs (as per the transfer event or the swap events) is almost exactly the same.
If there is not transfer event AND you see the following event:

```
└── Acknowledgement
        └── acknowledgement: 0x01
```

Then the transaction failed for some reason. If you instead see `acknowledgement: 0x00` the transaction executed correctly. Debugging such a transaction relies on using `.call_trace(True)`. Since this is an example and it isn't supposed to happen, we suggest quitting the interactive console and starting over.

### Slither

*[Slither](https://github.com/crytic/slither) is a Solidity static analysis framework written in Python 3. It runs a suite of vulnerability detectors, prints visual information about contract details, and provides an API to easily write custom analyses. Slither enables developers to find vulnerabilities, enhance their code comprehension, and quickly prototype custom analyses.*

Catalyst has been analyzed using Slither and no major bugs have been found. To rerun the analytics, run: `slither src`
