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

0x00: Asset Swap

```jsx
0 _context : Bytes[1]  # Used by CCI to detect how to unpack data
1-32 _fromPool : bytes32  # The origin pool
33-64 _pool : bytes32  # The target pool. Called by CCI on the other end.
65-96 _who : bytes32  # The recipient of the assets on the other end.
97-128 _U : uint256  # Number of units
129 _assetIndex : uint8  # Asset index on target pool
130-161 _minOut : uint256  # Minimum number of output assets.
162-193 _escrowAmount : uint256  # The number of tokens initially used.
194-225 _escrowToken : bytes32  # The token initially used.
226-227 _customDataLength : uint16  # If custom data is passed.
228-259+_customDataLength-32 _customData : bytes...  # The bytes passed to the custom Target.
The calldata target should be encoded within the first 32 bytes of _customData.
```

0x01: Liquidity Swap

```jsx
0 _context : Bytes[1]  # Used by CCI to detect how to unpack data
1-32 _fromPool : bytes32  # Where the call came from
33-64 _pool : bytes32  # The target pool. Called by CCI on the other end.
65-96 _who : bytes32  # The recipient of the assets on the other end.
97-128 _LU : uint256  # Number of liquidity units
129-160 _minOut : uint256
161-192 _escrowAmount : uint256  # The number of tokens initially used.
```


# Example of the data structure:
```py
import brownie
_chain = 1
_pool = brownie.convert.to_bytes("0x602C71e4DAC47a042Ee7f46E0aee17F94A3bA0B6".replace("0x00", ""))
_asset = 1
_who = brownie.convert.to_bytes("0x66aB6D9362d4F35596279692F0251Db635165871".replace("0x00", ""))
_U = int(1e18*251251*2**64)
```


`data = 0x0000000000000000000000000066ab6d9362d4f35596279692f0251db635165871000000000000000000000000602c71e4dac47a042ee7f46e0aee17f94a3ba0b600000000000000000000000066ab6d9362d4f35596279692f0251db635165871010000000000000000000000000000353458108326660000000000000000000000`

`data = 0x0000000000000000000000000066ab6d9362d4f35596279692f0251db635165871000000000000000000000000602c71e4dac47a042ee7f46e0aee17f94a3ba0b600000000000000000000000066ab6d9362d4f35596279692f0251db635165871010000000000000000000000000000353458108326660000000000000000000000`


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
    _CHECK = true; // <----
}
```
which breaks pool setup:
```solidity
function setupBase(
    string calldata name_,
    string calldata symbol_,
    address chaininterface,
    address setupMaster
) internal {
    // The pool is only designed to be used by a proxy and not as a standalone.
    // as a result self.check is set to TRUE on init, to stop anyone from using
    // the pool without a proxy.
    require(!_CHECK); // <----
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

### Vyper

To compile vyper contracts directly, the correct Vyper version should be installed independently of this project. eth-brownie depends on the newest version of Vyper, which the contracts might not be compatible with.

- Vyper
  - via pip: `pip install vyper==<version>`
  - via docker: [vyper.readthedocs.io](https://vyper.readthedocs.io/en/latest/installing-vyper.html#docker)
