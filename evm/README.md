# Catalyst

The EVM implementation of Catalyst is used as a reference implementation for other development efforts. The main logic is split into 6 contracts:

- `FixedPointMath.sol` : The mathematical library used for Catalyst. Implements the necessary mathematical functions the Catalyst documentation requires.
- `CatalystIBCInterface.sol` : Describes how the Catalyst protocol interfaces with the message router. This includes packing and unpacking of data and wrapping of various incoming calls.
- `SwapPoolCommon.sol` : Implements logic which doesn't depend on the swap curve. Containing the implementation in one contract allows for less repetition and a simplified development experience.
- `SwapPool.sol` : Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{W}{w \ln(2)}$.
- `SwapPoolAmplified.sol` : Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{1 - \theta}{w^\theta}$.
- `SwapPoolFactory.sol` : Simplifies deployment of swap pools through Open Zeppelin's Clones. Minimal proxies which uses delegate calls to the deployed contracts. This significantly reduces pool deployment cost. 

## FixedPointMath.sol

The mathematical library used to handle fixed point numbers. Fixed point numbers are  implemented transparently in `uint256` or `int128` by multiplying integers by $2^{64}$, such that $(4)_{64} = 4 \cdot 2^{64} = 73,786,976,294,838,206,464$. Similarly, decimal numbers can be represented: $(0.375)_{64} = 0.375 \cdot 2^{64} = 6,917,529,027,641,081,856$. The mathematical library contains functions which handle multiplication and division that could overflow in `uint256`.

The library enables computation of: 

- `log2X64`: Computes $\log_2$ for X64 input and output.

- `p2X64`: Computes $2^x$ for X64 input and output.

- `invp2X64`: Computes $2^{-x}$ for X64 input and output.

- `fpowX64`: Computes $x^y$ for X64 inputs and outputs. Uses the identity $2^{y \cdot \log_2x}$

- `invfpowX64`: Computes $x^{-y}$ for X64 inputs and outputs. Uses the identity $2^{-y \cdot \log_2x}$

$log_2$ only works for $x ≥ 1$. If $x < 1$, use the identity: $log_2(x) = - log_2(x^{-1}).$ Since $x^y$ is implemented through $log_2$, the similar identity can be used: $x^p = \left(\frac{1}{x}\right)^{-p}$.

## CatalystIBCInterface.sol

An intermediate contract between swap pools and the message router. This contract is specifically designed to sit between Catalyst swap pools and an IBC compliant message router.

Wraps the cross-chain calls into a byte array. The byte array depends on the message purpose. The message purpose can be found in the first byte of the transaction:

```jsx
Context Flag
    2^0: Asset (0) or Liquidity swap (1)
      1: Non-approximate (0) or approximate (1)
	  2: Unused # Proposed: Additional Payload for execution (1)
	  3: Unused
	  .
	  .
	  7: Unused
```

Generally, the message format is:
```jsx
0 _context : Bytes[1]  # Used by CII to unpack data.
1-32 _fromPool : bytes32  # The sending pool. Since CCI sits between the pools and the message router, the CCI cannot infer the pool from the message router.
33-64 _pool : bytes32  # The receiving pool. The payload target which the other CCI should deliver the data to.
...
```

Catalyst v1 implements 2 type of swaps, Asset swaps and Liquidity Swaps.

### 0x00: Asset Swap

If `(_context & 0x01) == 0`, then the message is an asset swap. It could be called a cross-chain asset swap but there is nothing to stop a user from Asset Swapping between 2 pools on the same chain. As a result, it would be more fitting to call it cross-pool swap.

The Asset Swap implements the general message format:

```jsx
0 _context : Bytes[1]
1-32 _fromPool : bytes32
33-64 _pool : bytes32
65-96 _toAccount : bytes32  # The recipient of the assets on the target chain.
97-128 _U : uint256  # Number of units
129 _assetIndex : uint8  # Asset index on target pool
130-161 _minOut : uint256  # Minimum number of output assets. If the pool returns less, the transaction should revert.
162-193 _escrowAmount : uint256  # The number of tokens initially used.
194-225 _escrowToken : bytes32  # The token initially used.
226-227 _customDataLength : uint16  # If custom data is passed, then length.
228-259+_customDataLength-32 _customData : bytes...  # The bytes passed to the custom Target.
The calldata target should be encoded within the first 32 bytes of _customData.
```
The message is hashed on the sending chain. This allows the escrow storage to be moved into the cross-chain message, only true escrow information can be submitted to the escrow logic.

### 0x01: Liquidity Swap
If `(_context & 0x01) == 1`, then the message is an liquidity swap. The purpose of liquidity swaps is to reduce the cost of acquiring an even distribution of liquidity. While the asset cost (through slippage) would be the same as getting an even distribution manually, the gas cost and number of interactions required could be substantially less.

This is done by converting the 4 actions:
1. Withdraw tokens
2. Convert tokens to units and transfer to target pool
3. Convert units to an even mix of tokens
4. Deposit the tokens into the pool.

into a single transaction.


The Liquidity Swap implements the general message format:

```jsx
0 _context : Bytes[1] 
1-32 _fromPool : bytes32
33-64 _pool : bytes32
65-96 _toAccount : bytes32  # The recipient of the pool tokens on the target chain.
97-128 _LU : uint256  # Number of units
129-160 _minOut : uint256  # Minimum number of pool tokens minted to `_toAccount`. If the pool returns less, the transaction should revert.
161-192 _escrowAmount : uint256  # The number of pools tokens initially used.
```

### Encoding or decoding a Catalyst message

Using brownie, the below code example shows how to encode and decode a Catalyst message.

```py
from brownie import convert, ZERO_ADDRESS

def payloadConstructor(
    _fromPool,
    _toPool,
    _toAccount,
    _U,
    _assetIndex=0,
    _minOut=0,
    _escrowAmount=0,
    _escrowToken=ZERO_ADDRESS,
    _context=convert.to_bytes(0, type_str="bytes1"),
):
    return (
        _context
        + convert.to_bytes(_fromPool, type_str="bytes32")
        + convert.to_bytes(_toPool, type_str="bytes32")
        + _toAccount
        + convert.to_bytes(_U, type_str="bytes32")
        + convert.to_bytes(_assetIndex, type_str="bytes1")
        + convert.to_bytes(_minOut, type_str="bytes32")
        + convert.to_bytes(_escrowAmount, type_str="bytes32")
        + convert.to_bytes(_escrowToken, type_str="bytes32")
        + convert.to_bytes(0, type_str="bytes2")
    )


def evmBytes32ToAddress(bytes32):
    return convert.to_address(bytes32[12:])


def decodePayload(data, decode_address=evmBytes32ToAddress):
    context = data[0]
    if context & 1:
        return {
            "_context": data[0],
            "_fromPool": decode_address(data[1:33]),
            "_toPool": decode_address(data[33:65]),
            "_toAccount": decode_address(data[65:97]),
            "_LU": convert.to_uint(data[97:129]),
            "_minOut": convert.to_uint(data[129:161]),
            "_escrowAmount": convert.to_uint(data[161:193])
        }
    customDataLength = convert.to_uint(data[226:228], type_str="uint16")
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
        "customDataLength": customDataLength,
        "_customDataTarget": decode_address(data[228:260]) if customDataLength > 0 else None,
        "_customData": data[260:260+customDataLength - 32] if customDataLength > 0 else None
    }

data = payloadConstructor("0x66aB6D9362d4F35596279692F0251Db635165871", "0x33A4622B82D4c04a53e170c638B944ce27cffce3", convert.to_bytes("0x0063046686E46Dc6F15918b61AE2B121458534a5"), 12786308645202655232)
```

`data = 0000000000000000000000000066ab6d9362d4f35596279692f0251db63516587100000000000000000000000033a4622b82d4c04a53e170c638b944ce27cffce30000000000000000000000000063046686e46dc6f15918b61ae2b121458534a5000000000000000000000000000000000000000000000000b17217f7d1cf7800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000`

## SwapPoolCommon.sol

A contract abstract, implementing logic which doesn't depend on the swap curve. Among these are:

- Security limit logic
- Pool administration
- Connection management
- Escrow logic

Swap Pools can inherit `SwapPoolCommon.sol` to automatically be compliant with IBC callbacks and the security limit. 

By inheriting `SwapPoolCommon.sol`, Swap Pools are deployed inactive:
```solidity
constructor() ERC20("", "") {
    _INITIALIZED = true; // <----
}
```
which breaks pool setup:
```solidity
function setupBase(
    string calldata name_,
    string calldata symbol_,
    address chainInterface,
    address setupMaster
) internal {
    // The pool is only designed to be used by a proxy and not as a standalone.
    // as a result self.check is set to TRUE on init, to stop anyone from using
    // the pool without a proxy.
    require(!_INITIALIZED); // <----
    ...
}
```
This makes it necessary to deploy a minimal proxy which uses the pool logic via delegateCall.


## SwapPool.sol

Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{W}{w \ln(2)}$. This approximates the constant product AMM, also called $x \cdot y = k$. The swap curve is known from Uniswap v2 and Balancer.

## SwapPoolAmplified.sol

Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{1}{w^\theta} \cdot (1-\theta)$. This flattens the swap curve such that the marginal price is closer to 1:1. The flattening depends on $\theta$, where $\theta = 0$ always delivers 1:1 swaps. This is similar to Stable Swap except that the swap is computed asynchronously instead of synchronously.

## SwapPoolFactory.sol

Both `SwapPool.sol` and `SwapPoolFactory.sol` are deployed disabled as a result of inheriting `SwapPoolCommon.sol`. To ease pool creation, `SwapPoolFactory.sol` wraps the deployment of minimal proxies and the associated setup of the Swap Pool in a single call.

# EVM Development

This repository uses Brownie for smart contract development, testing and deployment. Brownie can handle multiple versions of Solidity and Vyper and will automatically combine contracts to be deploy-ready. Brownie depends on `ganache`.

## Dev dependencies

- ganache-cli
  
  - `pnpm install -g ganache`

- eth-brownie
  
  - via [poetry](https://python-poetry.org)  (`brew install poetry`): `poetry install` in `/`
  - via pip: `pip3 install eth-brownie` (check that `$PATH` is properly configured).

- Python dependencies in *./pyproject.toml*. Automatically installed with `poetry install`
  
  - Note: You can set the poetry python version via `poetry env use python3.9` for example.

- Blockchain API
  
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

## Contracts

Contracts are stored in *./contracts*. Contracts compiled by brownie, `brownie compile` are stored in *./build*. Brownie will automatically download compatible solidity and vyper versions for internal usage.

### Solidity

To compile solidity contracts directly (not through Brownie), one has to install:

- Solidity
  
  - via brew: `brew tap ethereum/ethereum` then `brew install solidity`
  - via npm: `pnpm install -g solc` (installs solcjs)
  - [soliditylang.org](https://docs.soliditylang.org/en/latest/installing-solidity.html)

- `pnpm install`

- `solc <path-to-contract> --base-path node_modules`

### Slither

*[Slither](https://github.com/crytic/slither) is a Solidity static analysis framework written in Python 3. It runs a suite of vulnerability detectors, prints visual information about contract details, and provides an API to easily write custom analyses. Slither enables developers to find vulnerabilities, enhance their code comprehension, and quickly prototype custom analyses.*

Catalyst has been analyzed using Slither and no major bugs was found. To rerun the analytics, run:

`slither contracts/<>.sol --solc-remaps @openzeppelin=node_modules/@openzeppelin --solc-args "--optimize --optimize-runs 1000" --exclude naming-convention`

For each contract. `slither .` does not work.

### Vyper

To compile vyper contracts directly, the correct Vyper version should be installed independently of this project. eth-brownie depends on the newest version of Vyper, which the contracts might not be compatible with.

- Vyper
  - via pip: `pip install vyper==<version>`
  - via docker: [vyper.readthedocs.io](https://vyper.readthedocs.io/en/latest/installing-vyper.html#docker)
