# Catalyst Overview
Catalyst is structured in the following manner:
- **Vaults** holds assets on different chains and holds the logic for converting tokens into untis.
- **Factory** simplifies the deployment of new vaults.
- Cross-chain **interface** converts swap context into a message which can be sent cross-chain.

This structure is implemented on EVM as follows:
- `VaultCommon.sol` : Defines the structure of a Catalyst vault and implements logic that is common to all vaults.
  - `VaultVolatile.sol` : Extends `VaultCommon.sol` with the price curve $P(w) = \frac{W}{w}$.
  - `VaultAmplified.sol` : Extends `VaultCommon.sol` with the price curve $P(w) = \frac{1 - \theta}{w^\theta}$.
  - `FixedPointMathLib.sol` : The mathematical library used by Catalyst (based on the [solmate](https://github.com/transmissions11/solmate/blob/ed67feda67b24fdeff8ad1032360f0ee6047ba0a/src/utils/FixedPointMathLib.sol)).
- `CatalystVaultFactory.sol` : Simplifies the deployment of swap vaults via Open Zeppelin's *Clones*: vaults are deployed as minimal proxies which employ delegate calls to core contracts. This significantly reduces vault deployment cost.
- `CatalystIBCInterface.sol` : Bridges the Catalyst protocol with the message router of choice.

The EVM implementation is to be used as a reference implementation for further implementations.

# Catalyst Contracts
## SwapVaultCommon.sol

An `abstract` contract (i.e. a contract that is made to be overriden), which enforces the core structure of a Catalyst vault and implements features which are generic to any pricing curve. Among these are:

- Vault administration, including fees and vault connections management
- Cross chain swaps acknowledgement and timeout
- Security limit

Note that contracts derived from this one cannot be used directly but rather via proxy contracts. For this, `SwapVaultCommon.sol` implements [Initializable.sol](https://docs.openzeppelin.com/contracts/4.x/api/proxy#Initializable) to ensure that the vault proxies are correctly setup.

## SwapVaultVolatile.sol

Extends `SwapVaultCommon.sol` with the price curve $P(w) = \frac{W}{w}$. This approximates the constant product AMM (also called $x \cdot y = k$), mostly known from Uniswap v2 and Balancer.

## SwapVaultAmplified.sol

Extends `SwapVaultCommon.sol` with the price curve $P(w) = \frac{1}{w^\theta} \cdot (1-\theta)$. This introduces an argument $\theta$ which gives control over the flattening of the swap curve, such that the marginal price between assets is closer to 1:1 for a greater amount of swaps. With $\theta = 0$ the pool always delivers 1:1 swaps. This resembles Stable Swap, but with the advantage of allowing for asynchronous swaps.

## SwapVaultFactory.sol

`SwapVaultFactory.sol` handles the deployment and configuration of Catalyst vaults proxy contracts within a single call.

## CatalystIBCInterface.sol

An intermediate contract designed to interface Catalyst swap vaults with an IBC compliant messaging router. It wraps and unwraps the swaps calls to and from byte arrays so that they can be seamlessly sent and received by the router.

Catalyst v1 implements 2 type of swaps, *Asset Swaps* and *Liquidity Swaps*. The byte array specification for these can be found in `/contracts/CatalystIBCPayload.sol`.

- <u>`0x00`: Asset Swap</u><br/> Swaps with context `0x00` define asset swaps. Although primarily designed for cross-chain asset swaps, there is nothing from stopping a user of *Asset Swapping* between 2 vaults on the same chain.
- <u>`0x01`: Liquidity Swap</u><br/> Swaps with context `0x01` define liquidity swaps. These reduce the cost of rebalancing the liquidity distribution across vaults by combining the following steps into a single transaction:
  1. Withdraw tokens
  2. Convert tokens to units and transfer to target vault
  3. Convert units to an even mix of tokens
  4. Deposit the tokens into the vault.

For both kind of swaps, a *swap hash* derived from parts of the message is included on the cross-chain message. This serves to identify the swap on both the source and destination vaults, and for acknowledgment/timeout purposes on the source vault.

Refer to the helpers `encode_swap_payload` and `decode_payload` on `tests/catalyst/utils/vault_utils.py` for examples on how to encode and decode a Catalyst message.


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

This repository contains an example Catalyst deployment helper (found in `/scripts/deployCatalyst.py`). It handles the deployment of all the relevant Catalyst contracts, along with the tokens handling and vault creation. As an example, the steps to execute a local swap and a cross-vault swap (from and to the same vault) are further outlined:

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

Deploy Catalyst by invoking the helper `Catalyst(...)` from the imported script. This deploys all Catalyst contracts and creates a Catalyst vault.

```python
ps = Catalyst(acct, ibcinterface=ie)  # Deploys Catalyst
vault = ps.swapvault
tokens = ps.tokens
```

Transactions which require tokens to be transferred to the vault require the user to always approve the required allowance to the vault first. For this example, the vault is allowed an unbounded amount of tokens on behalf of the user.

```python
tokens[0].approve(vault, 2**256-1, {'from': acct})
```

## Execute a LocalSwap
The following transaction swaps 50 token0 for token1. A minimum output of 45 tokens is specified (if not fulfilled, the transaction will revert).

```python
localSwap_tx = vault.localSwap(tokens[0], tokens[1], 50 * 10**18, 45 * 10**18, {'from': acct})
```

## Cross-Chain Vault Setup
Before being able of executing a cross-chain swap, an IBC channel between vaults must be established. The following establishes a channel to and from the `CatalystIBCInterface.sol` contract, allowing cross-chain swaps between vaults which use this interface.

```python
# Registor IBC ports.
ps.crosschaininterface.registerPort()
ps.crosschaininterface.registerPort()
```

Once the cross-chain interface is properly connected, swaps between the test vault and itself can be allowed. Note that this does not represent a real use case scenario, as vault connections are to be created between different vaults and not within the same vault. However, this provides a simple manner in which to test the cross chain capabilities of Catalyst vaults.

```python
chid = convert.to_bytes(1, type_str="bytes32")  # Define the channel id to be 1. The emulator ignores this but it is important for the connection.

# Create the connection between the vault and itself:
vault.setConnection(
    chid,
    convert.to_bytes(vault.address.replace("0x", "")),
    True,
    {"from": acct}
)
```

## Execute a Cross Chain Swap
The following code swaps 10% of the vault value from token0 to token1 via the cross chain channel defined above.

```python
swap_amount = tokens[0].balanceOf(vault)//10
sendAsset_tx = vault.sendAsset(
    chid,
    convert.to_bytes(vault.address.replace("0x", "")),  # Set the target vault as itself. (encoded in bytes32)
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

All contracts are stored in *`./contracts`*. These can be compiled by brownie with `brownie compile`, which will save the compilation output to *`./build`*. Brownie will automatically download the required solidity and vyper compiler versions.

### Solidity

To compile solidity contracts directly (not through Brownie), perform the following steps:

- Install the Solidity compiler
  
  - via brew: `brew tap ethereum/ethereum` then `brew install solidity`
  - via npm: `pnpm install -g solc` (installs solcjs)
  - [soliditylang.org](https://docs.soliditylang.org/en/latest/installing-solidity.html)

- Install the required contract dependencies `pnpm install` (see the dev dependencies section of this README for further details).

- Compile the contracts with `solc <path-to-contract> --base-path . --include-path node_modules`

### Slither

*[Slither](https://github.com/crytic/slither) is a Solidity static analysis framework written in Python 3. It runs a suite of vulnerability detectors, prints visual information about contract details, and provides an API to easily write custom analyses. Slither enables developers to find vulnerabilities, enhance their code comprehension, and quickly prototype custom analyses.*

Catalyst has been analyzed using Slither and no major bugs have been found. To rerun the analytics, run:

`slither contracts/<>.sol --solc-args "--base-path . --include-path node_modules --optimize --optimize-runs 9000" --exclude naming-convention`

Alternativly, run `slither contracts` to analyze all contracts.
