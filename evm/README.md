# Catalyst

The EVM implementation of Catalyst is used as a reference implementation for other development efforts. The main logic is split into 6 contracts:

- `FixedPointMath.sol` : The mathematical library used for Catalyst. Implements the necessary mathematical functions the Catalyst documentation requires.
- `CatalystIBCInterface.sol` : Describes how the Catalyst protocol interfaces with the message router. This includes packing and unpacking of data and wrapping of various incoming calls.
- `SwapPoolCommon.sol` : Implements logic which doesn't depend on the swap curve. Containing the implementation in one contract allows for less repetition and a simplified development experience.
- `SwapPool.sol` : Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{W}{w \ln(2)}$.
- `SwapPoolAmplified.sol` : Extends `SwapPoolCommon.sol` with the price curve $P(w) = \frac{1 - \theta}{w^\theta}$.
- `SwapPoolFactory.sol` : Simplifies deployment of swap pools through Open Zeppelin's Clones. Minimal proxies which uses delegate calls to the deployed contracts. This significantly reduces pool deployment cost. 


## FixedPointMath.sol

Implements: 

- `log2X64`: Computes $\log_2$ for X64 input and output.

- `p2X64`: Computes $2^x$ for X64 input and output.

- `invp2X64`: Computes $2^{-x}$ for X64 input and output.

- `fpowX64`: Computes $x^y$ for X64 inputs and outputs. Uses the identity $2^{y · \log_2x}$
- 
- `invfpowX64`: Computes $x^{-y}$ for X64 inputs and outputs. Uses the identity $2^{-y · \log_2x}$

Power function only works for $base ≥ 1$. If $base <> 1$ use the identity: $base^p = \left(\frac{1}{base}\right)^{-p}$.

## CatalystIBCInterface.sol

An intermediate contract between swap pools and the message router. This contract is specifically designed to sit between Catalyst swap pools and and IBC compliant message router.

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

Deployed delegate proxies for `SwapPool.vy`

## SwapPool.sol

Provides `SwapPool.vy` with a Polymerase interface.

The data is the same between swap implementation (amplified/non-amplified). Data structure can be found as docstrings.

## SwapPoolAmplified.sol


## SwapPoolFactory.sol


