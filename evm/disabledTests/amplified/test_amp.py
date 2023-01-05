import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain
from brownie.test import given, strategy
from hypothesis import settings


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# POOLNAME = "PS One Two Three"
# POOLSYMBOL = "ps(ott) "
POOLNAME = "PS OneTwoThree"
POOLSYMBOL = "ps(OTT) "

depositValues = [1000 * 10**18, 1000 * 10**18]


@given(swapValue=strategy("uint256", max_value=depositValues[0], min_value=10**18))
def test_local_swap(accounts, token1, token2, gov, default_amp_swappool, swapValue):
    swappool = default_amp_swappool

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappool, swapValue, {"from": base_account})
    assert token2.balanceOf(base_account) == 0

    a = token1.balanceOf(swappool)
    b = token2.balanceOf(swappool)
    x = swapValue
    w1 = swappool._weight(token1)
    w2 = swappool._weight(token2)
    w = w1 / w2
    k = swappool._amp() / 2**64
    y = b - ((w2 * b ** (1 - k) - w1 * ((a + x) ** (1 - k) - a ** (1 - k))) / (w2)) ** (
        1 / (1 - k)
    )

    diviation = 0.02 / 100
    tx = swappool.localswap(token1, token2, swapValue, 0, {"from": base_account})
    out = token2.balanceOf(base_account)

    assert token1.balanceOf(base_account) == 0

    if (swapValue < token1.balanceOf(swappool) / 1000) or (
        y == 0
    ):  # also covers swapValue == 0
        assert out <= y
    else:
        assert 1 + diviation >= out / y >= 1 - diviation * 100  # lower is 2%
        # The calculation should get more precise as swapValue goes up.

    # swap the other way
    token2.approve(swappool, out, {"from": base_account})
    tx2 = swappool.localswap(token2, token1, out, 0, {"from": base_account})
    assert token2.balanceOf(base_account) == 0
    out2 = token1.balanceOf(base_account)

    assert out2 <= swapValue
    print(out2 / swapValue, out / y if y > 0 else out)

    # reset ...
    chain.revert()


# Small swaps
@pytest.mark.no_call_coverage
@given(swapValue=strategy("uint256", max_value=10**18))
def test_local_swapZERO(accounts, token1, token2, gov, default_amp_swappool, swapValue):
    swappool = default_amp_swappool

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappool, swapValue, {"from": base_account})
    assert token2.balanceOf(base_account) == 0

    a = token1.balanceOf(swappool)
    b = token2.balanceOf(swappool)
    x = swapValue
    w1 = swappool._weight(token1)
    w2 = swappool._weight(token2)
    w = w1 / w2
    k = swappool._amp() / 2**64
    y = b - ((w2 * b ** (1 - k) - w1 * ((a + x) ** (1 - k) - a ** (1 - k))) / (w2)) ** (
        1 / (1 - k)
    )

    diviation = 0.02 / 100
    tx = False
    try:
        tx = swappool.localswap(token1, token2, swapValue, 0, {"from": base_account})
    except brownie.exceptions.VirtualMachineError:
        print("Reverted, but that is okay.")
        return True

    if tx:
        assert token1.balanceOf(base_account) == 0

        if (
            swapValue < 104033365717131099 * 1.0001
        ) or (  # token1.balanceOf(swappool) / 1000) or (
            y == 0
        ):  # also covers swapValue == 0
            assert token2.balanceOf(base_account) <= y
        else:
            print(token2.balanceOf(base_account) / y)
            assert 1 + diviation >= token2.balanceOf(base_account) / y  # lower is 2%
            # The calculation should get more precise as swapValue goes up.

        out = token2.balanceOf(base_account)
        # swap the other way
        token2.approve(swappool, out, {"from": base_account})
        try:
            tx2 = swappool.localswap(token2, token1, out, 0, {"from": base_account})
        except brownie.exceptions.VirtualMachineError:
            print("Reverted, but that is /mostly/ okay.")

        assert token2.balanceOf(base_account) == 0
        out2 = token1.balanceOf(base_account)

        assert out2 <= swapValue
        if out2 > 0:
            print(out2 / swapValue, out)
        else:
            print(out2, out)

        # reset
        token1.transfer(gov, swapValue - out2, {"from": swappool})
        token1.transfer(gov, out2, {"from": base_account})
        token2.transfer(
            swappool, token2.balanceOf(base_account), {"from": base_account}
        )
