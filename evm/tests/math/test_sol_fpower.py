import pytest

import brownie

from brownie.test import given, strategy
from math import log2


il2_diviation = 1 / 2**20
pow2_diviation = 1 / 2**18


@given(value=strategy("uint256", min_value=2**64, max_value=2**192 - 1))
def test_self_pow(mathX64, value):

    calc = mathX64.ifpowX64(value, 2**64) / 2**64

    py = value / 2**64
    lower = py ** (1 - il2_diviation) * (1 - pow2_diviation)
    higher = py ** (1 + il2_diviation) * (1 + pow2_diviation)
    assert lower <= calc <= higher


@given(
    value=strategy("uint256", min_value=2**64, max_value=2**192 - 1),
    p=strategy("uint256", max_value=191 * 2**64 - 1),
)
def test_f_pow(mathX64, value, p):
    if log2(value / 2**64) * p >= 192 * 2**64:
        with brownie.reverts():
            mathX64.ifpowX64(value, p)
    else:
        calc = mathX64.ifpowX64(value, p) / 2**64
        py = (value / 2**64) ** (p / 2**64)
        lower = py ** (1 - il2_diviation) * (1 - pow2_diviation)
        higher = py ** (1 + il2_diviation) * (1 + pow2_diviation)
        assert lower <= calc <= higher
