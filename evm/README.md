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

This repository contains an example Catalyst deployment helper (found in `/scripts/deployCatalyst.py`). It handles the deployment of all the relevant Catalyst contracts, along with the tokens handling and pool creation. As an example, the steps to execute a local swap and a cross-pool swap (from and to the same pool) are further outlined:

## Catalyst Setup
Start by opening the Brownie interactive console. For simplicity, use a local ganache instance:

```bash
brownie console --network development
```

Import the relevant classes needed for the example:

```python
from scripts.deployCatalyst import Catalyst, decode_payload
from brownie import convert  # Used to convert between values and bytes.
```

Next, define the account that will be used to sign the transactions, and deploy the provided message router emulator. The emulator contains no message routing logic but rather it only simulates the execution of cross-chain packages.

```python
acct = accounts[0]  # Define the account used for testing

ie = IBCEmulator.deploy({'from': acct})  # Deploy the IBC emulator.
```

Deploy Catalyst by invoking the helper `Catalyst(...)` from the imported script. This deploys all Catalyst contracts and creates a Catalyst pool.

```python
ps = Catalyst(acct, ibcinterface=ie)  # Deploys Catalyst
pool = ps.swappool
tokens = ps.tokens
```

Transactions which require tokens to be transferred to the pool require the user to always approve the required allowance to the pool first. For this example, the pool is allowed an unbounded amount of tokens on behalf of the user.

```python
tokens[0].approve(pool, 2**256-1, {'from': acct})
```

## Execute a LocalSwap
The following transaction swaps 50 token0 for token1. A minimum output of 45 tokens is specified (if not fulfilled, the transaction will revert).

```python
localSwap_tx = pool.localSwap(tokens[0], tokens[1], 50 * 10**18, 45 * 10**18, {'from': acct})
```

## Cross-Chain Pool Setup
Before being able of executing a cross-chain swap, an IBC channel between pools must be established. The following establishes a channel to and from the `CatalystIBCInterface.sol` contract, allowing cross-chain swaps between pools which use this interface.

```python
# Registor IBC ports.
ps.crosschaininterface.registerPort()
ps.crosschaininterface.registerPort()
```

Once the cross-chain interface is properly connected, swaps between the test pool and itself can be allowed. Note that this does not represent a real use case scenario, as pool connections are to be created between different pools and not within the same pool. However, this provides a simple manner in which to test the cross chain capabilities of Catalyst pools.

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

## Execute a Cross Chain Swap
The following code swaps 10% of the pool value from token0 to token1 via the cross chain channel defined above.

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

The swap has been initiated, but the purchased tokens have not been yet sent to the user (this can be observed by inspecting `sendAsset_tx.info()`). This is because the cross-chain package has only been emitted but not yet executed. The relayer now has to collect the package and submit it to the target chain. Before doing this, the IBC payload can be examined to better understand what Catalyst is sending to the target chain:

```python
sendAsset_tx.events["IncomingPacket"]["packet"][3]
decode_payload(sendAsset_tx.events["IncomingPacket"]["packet"][3])
```

Finally, the IBC package can be executed as follows, making the user receive their purchased tokens.

```python
swap_execution_tx = ie.execute(sendAsset_tx.events["IncomingMetadata"]["metadata"][0], sendAsset_tx.events["IncomingPacket"]["packet"], {"from": acct})

swap_execution_tx.info()
```

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
