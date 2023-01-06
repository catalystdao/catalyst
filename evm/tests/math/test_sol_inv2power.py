import pytest

import brownie

from brownie import chain, accounts
from brownie.test import given, strategy
from hypothesis import settings
from math import sqrt, floor, exp, log, log2
import numpy as np
from decimal import Decimal


invpow2_diviation = 1 / 2**15


@given(value=strategy("uint256", max_value=50 * 2**64, min_value=2 ** (64 - 7)))
def test_inv2power(gov, mathX64, value):
    if value >= 41 * 2**64:
        with brownie.reverts():
            mathX64.iinvp2X64(value)
    else:
        calc = mathX64.iinvp2X64(value) / 2**64
        assert (
            calc * (1 - invpow2_diviation)
            <= 2 ** (-value / 2**64)
            <= calc * (1 + invpow2_diviation)
        )
