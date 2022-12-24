import pytest

import brownie

from brownie.test import given, strategy


pow2_diviation = 1 / 2**18


@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


@given(value=strategy("uint256", max_value=256 * 2**64 * 2))
def test_2power(mathX64, value):
    if value >= 192 * 2**64:
        with brownie.reverts():
            mathX64.ip2X64(value)
    else:
        calc = mathX64.ip2X64(value) / 2**64
        assert (
            calc * (1 - pow2_diviation)
            <= 2 ** (value / 2**64)
            <= calc * (1 + pow2_diviation)
        )
