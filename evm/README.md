# Catalyst Overview
Catalyst is structured in the following manner:
- Multiple Catalyst **pool** contracts that facilitate swaps between assets (either within or across pools).
- A Catalyst **factory** that is in charge of deploying pools.
- A Catalyst **interface** that facilitates communication between the pools and the message router of choice.

This structure is implemented on EVM as follows:
- `SwapPoolCommon.sol` : Defines the structure of a Catalyst pool and implements logic that is common to all pools.
  - `SwapPoolVolatile.sol` : Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{W}{w}$.
  - `SwapPoolAmplified.sol` : Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{1 - \theta}{w^\theta}$.
  - `FixedPointMathLib.sol` : The mathematical library used by Catalyst (based on the [solmate](https://github.com/transmissions11/solmate/blob/ed67feda67b24fdeff8ad1032360f0ee6047ba0a/src/utils/FixedPointMathLib.sol)).
- `SwapPoolFactory.sol` : Simplifies the deployment of swap pools via Open Zeppelin's *Clones*: pools are deployed as minimal proxies which employ delegate calls to core contracts. This significantly reduces pool deployment cost.
- `CatalystIBCInterface.sol` : Bridges the Catalyst protocol with the message router of choice.

The EVM implementation is to be used as a reference implementation for further implementations.

# Catalyst Contracts
## SwapPoolCommon.sol

An `abstract` contract (i.e. a contract that is made to be overriden), which enforces the core structure of a Catalyst pool and implements features which are generic to any pricing curve. Among these are:

- Pool administration, including fees and pool connections management
- Cross chain swaps acknowledgement and timeout
- Security limit

Note that contracts derived from this one cannot be used directly but rather via proxy contracts. For this, `SwapPoolCommon.sol` implements [Initializable.sol](https://docs.openzeppelin.com/contracts/4.x/api/proxy#Initializable) to ensure that the pool proxies are correctly setup.

## SwapPoolVolatile.sol

Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{W}{w}$. This approximates the constant product AMM (also called $x \cdot y = k$), mostly known from Uniswap v2 and Balancer.

## SwapPoolAmplified.sol

Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{1}{w^\theta} \cdot (1-\theta)$. This introduces an argument $\theta$ which gives control over the flattening of the swap curve, such that the marginal price between assets is closer to 1:1 for a greater amount of swaps. With $\theta = 0$ the pool always delivers 1:1 swaps. This resembles Stable Swap, but with the advantage of allowing for asynchronous swaps.

## SwapPoolFactory.sol

`SwapPoolFactory.sol` handles the deployment and configuration of Catalyst pools proxy contracts within a single call.

## CatalystIBCInterface.sol

An intermediate contract designed to interface Catalyst swap pools with an IBC compliant messaging router. It wraps and unwraps the swaps calls to and from byte arrays so that they can be seamlessly sent and received by the router.

Catalyst v1 implements 2 type of swaps, *Asset Swaps* and *Liquidity Swaps*. The byte array specification for these can be found in `/contracts/CatalystIBCPayload.sol`.

- <u>`0x00`: Asset Swap</u><br/> Swaps with context `0x00` define asset swaps. Although primarily designed for cross-chain asset swaps, there is nothing from stopping a user of *Asset Swapping* between 2 pools on the same chain.
- <u>`0x01`: Liquidity Swap</u><br/> Swaps with context `0x01` define liquidity swaps. These reduce the cost of rebalancing the liquidity distribution across pools by combining the following steps into a single transaction:
  1. Withdraw tokens
  2. Convert tokens to units and transfer to target pool
  3. Convert units to an even mix of tokens
  4. Deposit the tokens into the pool.

For both kind of swaps, a *swap hash* derived from parts of the message is included on the cross-chain message. This serves to identify the swap on both the source and destination pools, and for acknowledgment/timeout purposes on the source pool.

Refer to the helpers `encode_swap_payload` and `decode_payload` on `tests/catalyst/utils/pool_utils.py` for examples on how to encode and decode a Catalyst message.



# EVM Development

This repository uses Brownie for the development, testing and deployment of the smart contracts. Brownie can handle multiple versions of Solidity and Vyper and will automatically combine contracts to be deploy-ready.

## Dev dependencies
Not that the following dependencies have been tested to work with `python3.9`. The installation steps included here are for reference only, plese refer to the specific documentation of each of the mentioned packages for further information.

- Install the `ganache-cli` globaly (required by `brownie`)
  - `pnpm install -g ganache`

- Install the contract templates [@openzeppelin/contracts](https://www.npmjs.com/package/@openzeppelin/contracts) and [Solmate](https://www.npmjs.com/package/solmate)
  - `pnpm install` (run from the `evm` root directory)

- Install `eth-brownie`
  - via `pip`: `pip3 install eth-brownie` (check that `$PATH` is properly configured).
  - via [`poetry`](https://python-poetry.org):
    - If `poetry` is not installed on your system, use `brew install poetry`
    - Set the `poetry` python version with `poetry env use python3.9`.
    - `poetry install` (run from the `evm` root directory). This will install all the dependencies specified in `./pyproject.toml`.

### Further dependencies
- To deploy Catalyst on testnets:
  
  - Default: [alchemy](https://www.alchemy.com), export key to `$ALCHEMY_API_TOKEN`
  
  - Alt: [Infura](https://infura.io), edit *./.brownie/network-config.yaml* with Infura RPC.

# Development with Brownie 

Catalyst contains a demonstration deployment script. It handles the deployment of the relevant Catalyst contracts, along with tokens and deploying a Catalyst pool.

The script can be found in `/scripts/deployCatalyst.py`. We will demonstrate how to execute a local swap and a cross-pool swap (from and to the same pool).

Start by opening the Brownie interactive console. For simplicity, we will use a local ganache instance:

```bash
brownie console --network development
```

Import the relevant classes needed for the example:

```python
from scripts.deployCatalyst import Catalyst, decode_payload
from brownie import convert  # Used to convert between values and bytes.
```

Next, we will define an account and deploy a message router emulator. The emulator contains no message router logic, except emitting cross-chain packages and facilitates execution of cross-chain packages.

```python
acct = accounts[0]  # Define the account used for testing

ie = IBCEmulator.deploy({'from': acct})  # Deploy the IBC emulator.
```

Let's deploy Catalyst. This is done by calling `Catalyst(...)` from the imported script. This deploys all Catalyst contracts and creates an example pool for us.

```python
ps = Catalyst(acct, ibcinterface=ie)  # Deploys Catalyst
pool = ps.swappool
tokens = ps.tokens
```

For any contract interaction, the user needs to approve the pool to spend tokens.

```python
tokens[0].approve(pool, 2**256-1, {'from': acct})
```

Let's execute a localSwap. This swaps 50 token0 for token1. We also set a minimum output of 45 tokens, if less than 45 tokens are returned the swap reverts.

```python
localSwap_tx = pool.localSwap(tokens[0], tokens[1], 50 * 10**18, 45 * 10**18, {'from': acct})
```

Let's execute a cross-chain swap. Before we can do it, we need to connect the cross-chain interface with itself. This establishes a channel between the 2 ports allowing IBC messagse flow.

```python
# Registor IBC ports.
ps.crosschaininterface.registerPort()
ps.crosschaininterface.registerPort()
```

Once the cross-chain interface is properly connected, we can allow the pool to swap with itself. For a true deployment, connections would be created between different pools not from and to the same pool. But for the sake of simplicity, let's create a pool which connects with itself.

```python
chid = convert.to_bytes(1, type_str="bytes32")  # Define the channel id to be 1. The emulator ignores this but it is important for the connection.

# Create the connection between the pool and itself:
pool.setConnection(
    chid,
    convert.to_bytes(pool.address.replace("0x", "")),
    True,
    {"from": acct}
)
```

We can now execute a swap. We will swap 10% of the pool value, through the channel we defined earlier.

```python
swap_amount = tokens[0].balanceOf(pool)//10
sendAsset_tx = pool.sendAsset(
    chid,
    convert.to_bytes(pool.address.replace("0x", "")),  # Set the target pool as itself. (encoded in bytes32)
    convert.to_bytes(acct.address.replace("0x", "")),  # Set the target user as acct.   (encoded in bytes32)
    tokens[0],  # Swap out of token0.
    1,  # Swap into token1.
    swap_amount,  # Swap swap_amount of token0.
    30 * 10**18,  # Return more than 30 tokens.
    acct,  # If the transaction reverts, send the tokens back to acct.
    {"from": acct},  # Acct pays for the transactions.
)
```

The swap has now been initiated. But if you check `sendAsset_tx.info()` you will see that no tokens have been sent to the user. That makes sense since the cross-chain package has only been emitted but not executed yet. No relayer has collected the package and submitted it to the target chain. We can examine the payload to understand what Catalyst sends to the target chain:

```python
sendAsset_tx.events["IncomingPacket"]["packet"][3]
decode_payload(sendAsset_tx.events["IncomingPacket"]["packet"][3])
```

Let's execute the IBC package.

```python
swap_execution_tx = ie.execute(sendAsset_tx.events["IncomingMetadata"]["metadata"][0], sendAsset_tx.events["IncomingPacket"]["packet"], {"from": acct})

swap_execution_tx.info()
```

The user finally gets their tokens.

## Contracts

Contracts are stored in *./contracts*. Contracts compiled by brownie, `brownie compile` are stored in *./build*. Brownie will automatically download compatible solidity and vyper versions for internal usage.

### Solidity

To compile solidity contracts directly (not through Brownie), one has to install:

- Solidity
  
  - via brew: `brew tap ethereum/ethereum` then `brew install solidity`
  - via npm: `pnpm install -g solc` (installs solcjs)
  - [soliditylang.org](https://docs.soliditylang.org/en/latest/installing-solidity.html)

- `pnpm install`

- `solc <path-to-contract> --base-path . --include-path node_modules`

### Slither

*[Slither](https://github.com/crytic/slither) is a Solidity static analysis framework written in Python 3. It runs a suite of vulnerability detectors, prints visual information about contract details, and provides an API to easily write custom analyses. Slither enables developers to find vulnerabilities, enhance their code comprehension, and quickly prototype custom analyses.*

Catalyst has been analyzed using Slither and no major bugs was found. To rerun the analytics, run:

`slither contracts/<>.sol --solc-args "--base-path . --include-path node_modules --optimize --optimize-runs 9000" --exclude naming-convention`

Alternativly, run `slither contracts` to analyse all contracts.
