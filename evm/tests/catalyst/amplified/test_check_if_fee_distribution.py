import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain, CatalystSwapPoolFactory


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


def test_set_new_pool_fee(gov, accounts, default_amp_swappool_self):
    swappool = default_amp_swappool_self

    factoryOwner = gov
    assert CatalystSwapPoolFactory.at(swappool._factory()).owner() == factoryOwner

    feeAdministrator = accounts[1]
    swappool.setFeeAdministrator(feeAdministrator, {"from": factoryOwner})

    assert swappool._feeAdministrator() == feeAdministrator

    poolFee = 2**61
    swappool.setPoolFee(
        poolFee, {"from": feeAdministrator}
    )  # set to 12.5% (high for testing purposes)

    assert swappool._poolFeeX64() == poolFee


depositValue = 10**18 * 100


def test_deposit_into_pool(gov, accounts, default_amp_swappool_self, token1, token2):
    swappool = default_amp_swappool_self

    balance_modifier = accounts[2]

    token1.transfer(balance_modifier, depositValue, {"from": gov})
    token1.approve(swappool, depositValue, {"from": balance_modifier})
    token2.transfer(balance_modifier, depositValue, {"from": gov})
    token2.approve(swappool, depositValue, {"from": balance_modifier})

    baseAmount = (
        int((depositValue * swappool.totalSupply()) / token1.balanceOf(swappool)) - 1000
    )
    swappool.depositMixed([depositValue], baseAmount-1, {"from": balance_modifier})

    assert 10**5 > token1.balanceOf(balance_modifier)
    assert swappool.balanceOf(balance_modifier) == baseAmount


def test_swap_around_with_fees(
    gov, accounts, default_amp_swappool_self, token1, token2
):
    swappool = default_amp_swappool_self
    swapValue = 10**18 * 1000

    assert swappool._poolFeeX64() == 2**61

    # Instead of actually swapping, lets just transfer tokens to the pool.
    # This emulates swapping one way and then the other and balancing the pool.
    poolEarnings = (swapValue * swappool._poolFeeX64()) >> 64
    token1.transfer(swappool, poolEarnings, {"from": gov})
    token2.transfer(swappool, poolEarnings, {"from": gov})


def test_withdraw(gov, accounts, default_amp_swappool_self, token1, token2):
    swappool = default_amp_swappool_self

    balance_modifier = accounts[2]

    baseAmount = swappool.balanceOf(balance_modifier)

    assert 10**5 > token1.balanceOf(balance_modifier)
    assert 10**5 > token2.balanceOf(balance_modifier)

    swappool.withdrawAll(baseAmount, {"from": balance_modifier})

    assert depositValue + 10**5 < token1.balanceOf(balance_modifier)
    assert depositValue + 10**5 < token2.balanceOf(balance_modifier)
    # So fees are distributed without calling distributeFees.
