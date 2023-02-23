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

A contract abstract, implementing logic which doesn't depend on the swap curve. Among these are:

- Security limit logic
- Pool administration
- Connection management
- Escrow logic

Swap Pools can inherit `SwapPoolCommon.sol` to automatically be compliant with IBC callbacks and the security limit. 

`SwapPoolCommon.sol` implements [Initializable.sol](https://docs.openzeppelin.com/contracts/4.x/api/proxy#Initializable) to ensure the pool is correctly setup.

## SwapPoolVolatile.sol

Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{W}{w}$. This approximates the constant product AMM, also called $x \cdot y = k$. The swap curve is known from Uniswap v2 and Balancer.

## SwapPoolAmplified.sol

Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{1}{w^\theta} \cdot (1-\theta)$. This flattens the swap curve such that the marginal price is closer to 1:1. The flattening depends on $\theta$, where $\theta = 0$ always delivers 1:1 swaps. This is similar to Stable Swap except that the swap is computed asynchronously instead of synchronously.

## SwapPoolFactory.sol

Both `SwapPoolVolatile.sol` and `SwapPoolFactory.sol` are deployed disabled as a result of inheriting `SwapPoolCommon.sol`. To ease pool creation, `SwapPoolFactory.sol` wraps the deployment of minimal proxies and the associated setup of the Swap Pool in a single call.

## CatalystIBCInterface.sol

An intermediate contract between swap pools and the message router. This contract is specifically designed to sit between Catalyst swap pools and an IBC compliant message router.

Wraps the cross-chain calls into a byte array. The byte array depends on the message purpose. The message packing can be found in `/contracts/CatalystIBCPayload.sol`

Catalyst v1 implements 2 type of swaps, Asset swaps and Liquidity Swaps.

### 0x00: Asset Swap

If `_context == 0x00`, then the message is an asset swap. It could be called a cross-chain asset swap but there is nothing to stop a user from Asset Swapping between 2 pools on the same chain.

Parts of the message is hashed on the sending chain. This allows the escrow storage to be moved into the cross-chain message. This allows the smart contract to validate escrow information coming from the router.

### 0x01: Liquidity Swap
If `_context == 0x01`, then the message is an liquidity swap. The purpose of liquidity swaps is to reduce the cost of acquiring an even distribution of liquidity. While the asset cost (through slippage) would be the same as getting an even distribution manually, the gas cost and number of interactions required could be substantially less.

This is done by converting the 4 actions:
1. Withdraw tokens
2. Convert tokens to units and transfer to target pool
3. Convert units to an even mix of tokens
4. Deposit the tokens into the pool.

into a single transaction.

### Encoding or decoding a Catalyst message

Using brownie, the below code example shows how to encode and decode a Catalyst message.

```py
from brownie import convert, ZERO_ADDRESS

def encode_swap_payload(
    from_pool,
    to_pool,
    to_account,
    U,
    asset_index=0,
    min_out=0,
    escrow_amount=0,
    escrow_token=ZERO_ADDRESS,
    block_number=0
):

    return (
        convert.to_bytes(0, type_str="bytes1")
        + convert.to_bytes(from_pool, type_str="bytes32")
        + convert.to_bytes(to_pool, type_str="bytes32")
        + convert.to_bytes(to_account, type_str="bytes32")
        + convert.to_bytes(U, type_str="bytes32")
        + convert.to_bytes(asset_index, type_str="bytes1")
        + convert.to_bytes(min_out, type_str="bytes32")
        + convert.to_bytes(escrow_amount, type_str="bytes32")
        + convert.to_bytes(escrow_token, type_str="bytes32")
        + convert.to_bytes(block_number, type_str="bytes4")
        + convert.to_bytes(
            compute_asset_swap_hash(to_account, U, escrow_amount, escrow_token, block_number),
            type_str="bytes32"
        )
        + convert.to_bytes(0, type_str="bytes2")
    )


def evm_bytes_32_to_address(bytes32):
    return convert.to_address(bytes32[12:])


def decode_payload(data, decode_address=evm_bytes_32_to_address):

    context = data[0]

    # Liquidity swap payload
    if context & 1:
        return {
            "_context": data[0],
            "_fromPool": decode_address(data[1:33]),
            "_toPool": decode_address(data[33:65]),
            "_toAccount": decode_address(data[65:97]),
            "_LU": convert.to_uint(data[97:129]),
            "_minOut": convert.to_uint(data[129:161]),
            "_escrowAmount": convert.to_uint(data[161:193]),
            "_blockNumber": convert.to_uint(data[193:197]),
            "_swapHash": data[197:228],
        }
    
    # Asset swap payload
    custom_data_length = convert.to_uint(data[262:264], type_str="uint16")
    return {
        "_context": data[0],
        "_fromPool": decode_address(data[1:33]),
        "_toPool": decode_address(data[33:65]),
        "_toAccount": decode_address(data[65:97]),
        "_U": convert.to_uint(data[97:129]),
        "_assetIndex": convert.to_uint(data[129], type_str="uint8"),
        "_minOut": convert.to_uint(data[130:162]),
        "_escrowAmount": convert.to_uint(data[162:194]),
        "_escrowToken": decode_address(data[194:226]),
        "_blockNumber": convert.to_uint(data[226:230]),
        "_swapHash": data[230:262],
        "customDataLength": custom_data_length,
        "_customDataTarget": decode_address(data[264:296]) if custom_data_length > 0 else None,
        "_customData": data[296:296+custom_data_length - 32] if custom_data_length > 0 else None
    }

data = payloadConstructor("0x66aB6D9362d4F35596279692F0251Db635165871", "0x33A4622B82D4c04a53e170c638B944ce27cffce3", convert.to_bytes("0x0063046686E46Dc6F15918b61AE2B121458534a5"), 12786308645202655232)
```

`data = 0000000000000000000000000066ab6d9362d4f35596279692f0251db63516587100000000000000000000000033a4622b82d4c04a53e170c638b944ce27cffce30000000000000000000000000063046686e46dc6f15918b61ae2b121458534a5000000000000000000000000000000000000000000000000b17217f7d1cf7800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000`


# EVM Development

This repository uses Brownie for smart contract development, testing and deployment. Brownie can handle multiple versions of Solidity and Vyper and will automatically combine contracts to be deploy-ready. Brownie depends on `ganache`.

## Dev dependencies

- ganache-cli
  
  - `pnpm install -g ganache`

- [@openzeppelin/contracts](https://www.npmjs.com/package/@openzeppelin/contracts) and [Solmate](https://www.npmjs.com/package/solmate)
  - `pnpm install`

- eth-brownie
  
  - via [poetry](https://python-poetry.org)  (`brew install poetry`): `poetry install` in `/`
  - via pip: `pip3 install eth-brownie` (check that `$PATH` is properly configured).

- Python dependencies in *./pyproject.toml*. Automatically installed with `poetry install`
  
  - Note: You can set the poetry python version via `poetry env use python3.9` for example.

- To deploy Catalyst on testnets: a blockchain API
  
  - Default: [alchemy](https://www.alchemy.com), export key to `$ALCHEMY_API_TOKEN`
  
  - Alt: [Infura](https://infura.io), edit *./.brownie/network-config.yaml* with Infura RPC.

# Introduction to Brownie 

Brownie wraps smart contract development in a neat package. To deploy, fund an account loaded into Brownie:

- `brownie accounts --help`
  
  - `brownie accounts new <NAME OF ACCOUNT>`
    
    - Example: `brownie accounts new deployment` or `brownie accounts new 0` and provide a privatekey.
  
  - `brownie generate new <NAME OF ACCOUNT>`

- Fund the account generated by brownie. [Kovan faucet](https://github.com/kovan-testnet/faucet), [Rinkeby faucet](https://faucet.rinkeby.io).

Open the brownie dev console:

`brownie console --network <mainnet/kovan/development>`

and load the account:

```
acct = accounts.load('<NAME OF ACCOUNT>')
SC = SmartContractName.deploy(*init_vars, {'from': acct})
SC
```

The smart contract has now been deployed. Deployment scripts can be found in `./scripts/deploy/*`

## First deployment and swap

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
