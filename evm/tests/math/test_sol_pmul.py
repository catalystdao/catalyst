import pytest

import brownie

from brownie.test import given, strategy


@given(value=strategy("uint256[2]"))
def test_pMul(mathX64, value):
    # The largest number we can handle because of the decimals is
    # 2^(256-64) - 1 which is (2^96-1) · (2^96-1). So the max_value
    # We are testing from is then (2^(96+64)-1) = (2^(161)-1)
    # This becomes an issue before any other issue. (Like over overflow in r1)
    # We also should never let sufficient small numbers be entered
    # into the function, as it would be rounded down. This is not an issue we
    # are testing for, since it will not be relevant for our usages.
    if value[0] * value[1] >= (2 ** (161) - 1) ** 2:
        with brownie.reverts():
            mathX64.imul(value[0], value[1])
    else:
        calc = mathX64.imul(value[0], value[1])

        assert calc == (value[0] * value[1]) >> 64


@given(value=strategy("uint256[2]", max_value=2**64 - 1, min_value=0))
def test_pMul_low(mathX64, value):
    if value[0] * value[1] >= (2 ** (161) - 1) ** 2:
        with brownie.reverts():
            mathX64.imul(value[0], value[1])
    else:
        calc = mathX64.imul(value[0], value[1])

        assert calc == (value[0] * value[1]) >> 64


@given(value=strategy("uint256[2]", max_value=2**256 - 1, min_value=2**32))
def test_pMul_high(mathX64, value):
    if value[0] * value[1] >= (2 ** (161) - 1) ** 2:
        with brownie.reverts():
            mathX64.imul(value[0], value[1])
    else:
        calc = mathX64.imul(value[0], value[1])

        assert calc == (value[0] * value[1]) >> 64
