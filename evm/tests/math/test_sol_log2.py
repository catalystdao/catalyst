import pytest

import brownie

from brownie.test import given, strategy
import numpy as np

il2_diviation = 1 / 2**20


@given(value=strategy("uint256", max_value=2**256 - 1, min_value=2**64))
def test_l2(gov, mathX64, value):
    X64 = 2**64
    if value != 0:
        calc = mathX64.ilog2X64(value, {"from": gov})
        if calc < 2**64:  # The diviation is on calc, but
            assert (
                np.power(2, calc / X64) * (1 - il2_diviation) * X64
                <= value
                <= np.power(2, calc / X64) * (1 + il2_diviation) * X64
            )
        else:
            assert (
                np.power(2, calc / X64 * (1 - il2_diviation)) * X64
                <= value
                <= np.power(2, calc / X64 * (1 + il2_diviation)) * X64
            )

    else:
        with brownie.reverts():
            mathX64.ilog2X64(0, {"from": gov})
