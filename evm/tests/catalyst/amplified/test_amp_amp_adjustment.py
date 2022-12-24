import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, Token, SwapPool, chain
from brownie.test import given, strategy
from hypothesis import settings
from a_common_functions import get_swap_return, check_swap_return, return_swap_check
from math import ceil, floor


@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


ONEWEEK = 60 * 60 * 24 * 7
TWODAYS = 60 * 60 * 24 * 2


def test_only_administrator(gov, accounts, default_amp_swappool):
    swappool = default_amp_swappool

    startTime = chain.time()
    with brownie.reverts():
        swappool.modifyAmplification(
            startTime + ONEWEEK, 2**61, {"from": accounts[2]}
        )

    swappool.modifyAmplification(startTime + ONEWEEK, 2**61, {"from": gov})


def test_2_days_minimum(gov, default_amp_swappool):
    swappool = default_amp_swappool

    with brownie.reverts():
        swappool.modifyAmplification(chain.time() + TWODAYS - 1, 2**61, {"from": gov})

    swappool.modifyAmplification(chain.time() + TWODAYS + 1, 2**61, {"from": gov})


def test_adjustment_leads_to_uneven(gov, default_amp_swappool):
    swappool = default_amp_swappool

    newtime = chain.time() + ONEWEEK + 10
    swappool.modifyAmplification(newtime, 2**61, {"from": gov})
    swappool._adjustmentTarget() == newtime + 1
    swappool.modifyAmplification(newtime + 5, 2**61, {"from": gov})
    swappool._adjustmentTarget() == newtime + 5


def test_decrease_amp(gov, default_amp_swappool, token1, token2, token3, chain):
    swappool = default_amp_swappool

    currentAmp = swappool._amp()

    startTime = chain.time()
    swappool.modifyAmplification(startTime + ONEWEEK + 1, 2**61, {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()

    targetAmp = swappool._targetAmplification()
    
    assert swappool._targetAmplification() == 2**61

    chain.mine(1, timestamp=int(startTime + (ONEWEEK + 1) / 2))
    
    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = swappool._lastModificationTime()
    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    tx = swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    passedTime = (tx.timestamp - lastModification)/(duration)

    assert ceil(currentAmp * (1 - passedTime) + targetAmp * (passedTime))*1.0001 >= swappool._amp() >= ceil(currentAmp * (1 - passedTime) + targetAmp * (passedTime)) * 0.9999

    chain.mine(1, timestamp=int(startTime + (ONEWEEK + 1 + 5)))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._amp() == targetAmp


def test_increase_amp(gov, default_amp_swappool, token1, token2, token3, chain):
    swappool = default_amp_swappool

    currentAmp = swappool._amp()

    startTime = chain.time()
    swappool.modifyAmplification(startTime + (ONEWEEK + 1), 2**63, {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()

    assert swappool._targetAmplification() == 2**63
    targetAmp = swappool._targetAmplification()

    chain.snapshot()

    chain.mine(1, timestamp=int(startTime + (ONEWEEK + 1) / 2))
    
    lastModification = swappool._lastModificationTime()
    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    tx = swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    passedTime = (tx.timestamp - lastModification)/(duration)

    assert floor(currentAmp * (1 - passedTime) + targetAmp * (passedTime))*1.0001 >= swappool._amp() >= floor(currentAmp * (1 - passedTime) + targetAmp * (passedTime)) * 0.9999

    chain.mine(1, timestamp=int(startTime + (ONEWEEK + 5)))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._amp() == targetAmp
