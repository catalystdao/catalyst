import pytest

import brownie

from brownie.test import given, strategy
from math import log2


@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


invpow_diviation = 100 / 100  # TODO: Improve


@given(
    value=strategy("uint256", min_value=2**64 + 2**40, max_value=12 * 2**64),
    p=strategy("uint256", min_value=2**61 + 2**60, max_value=10**9 * 2**64),
)
def test_invp_pow(gov, mathX64, value, p):
    if log2(value / 2**64) * p >= 41 * 2**64:
        with brownie.reverts():
            mathX64.iinvfpowX64(value, p)
    else:
        calc = mathX64.iinvfpowX64(value, p) / 2**64
        py = np.power(value / 2**64, -p / 2**64)
        assert py * (1 - invpow_diviation) <= calc <= py * (1 + invpow_diviation)
