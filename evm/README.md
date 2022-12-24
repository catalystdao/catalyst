# Catalyst

Catalyst consists of many contracts. Full overview can be found in the[ Catalyst documentation folder](https://drive.google.com/drive/folders/1GVMRXfmL5kjmmYKEmUnghoiQxVijn68i?usp=sharing).

The 4 most important are `SwapPool.vy`, `SwapPoolFactory.vy`, `FixedPointMathX64.vy` and `CrossChainSwapInterface.vy`

## FixedPointMathX64.vy

Implements: 

- `pMULX64`, `pMULX64_abc`: Multiplication of 2 to 3 X64 numbers.

- `bigdiv64`: Computes (a << 64)/b, relevant for dividing 2 X64 numbers with X64 as output

- `log2X64`: Computes $\log_2$ for X64 input and output.

- `lnX64`: Computes natural logarithm for X64 input and output.

- `p2X64`: Computes $2^x$ for X64 input and output.

- `fpowX64`: Computes $x^y$ for X64 inputs and outputs via the identity $2^{y · \log_2x}$

- `expX64`: Computes $\exp x$ for X64 input and output.

No power function works for $base < 1$. Instead use the identity: $base^p = \left(\frac{1}{base}\right)^{-p}$ , with the modifier **inv** like: `invfpowX64`.

## SwapPool.vy

Uses FixedPointMathX64 to compute the relevant swap equation.

## SwapPoolFactory.vy

Deployed delegate proxies for `SwapPool.vy`

## CrossChainSwapInterface.vy

Provides `SwapPool.vy` with a Polymerase interface.

The data is the same between swap implementation (amplified/non-amplified). Data structure can be found as docstrings.


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


## Steps to Run

- Follow steps in the catalyst-core [README.md](../../README.md) to ensure dependencies are setup.
- `pip3 install numpy`
- `brownie run scripts/easyCatalyst.py`
- `brownie test tests/catalyst`


