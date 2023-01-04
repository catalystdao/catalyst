import brownie
import numpy as np
import pytest
from brownie import chain
from brownie.test import given, strategy
from hypothesis import settings
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
        swappool.modifyWeights(startTime + ONEWEEK, [2, 3], {"from": accounts[2]})

    swappool.modifyWeights(startTime + ONEWEEK, [2, 3], {"from": gov})


def test_2_days_minimum(gov, default_amp_swappool):
    swappool = default_amp_swappool

    with brownie.reverts():
        swappool.modifyWeights(chain.time() + TWODAYS - 1, [2, 3], {"from": gov})

    swappool.modifyWeights(chain.time() + TWODAYS + 1, [2, 3], {"from": gov})

def test_adjustment_leads_to_even(gov, default_amp_swappool):
    swappool = default_amp_swappool

    newtime = chain.time() + ONEWEEK + 10
    swappool.modifyAmplification(newtime, 2**61, {"from": gov})
    swappool._adjustmentTarget() == newtime
    swappool.modifyAmplification(newtime + 5, 2**61, {"from": gov})
    swappool._adjustmentTarget() == newtime + 6
    

def test_increase_weights(gov, default_amp_swappool, token1, token2, chain):
    swappool = default_amp_swappool

    currentWeights = [swappool._weight(tkn) for tkn in [token1, token2]]

    startTime = chain.time()
    swappool.modifyWeights(startTime + ONEWEEK, [20, 30], {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()
    
    targetWeight = [swappool._targetWeight(tkn) for tkn in [token1, token2]]

    # Weights should not change immediately.
    assert swappool._weight(token1) == currentWeights[0]
    assert swappool._weight(token2) == currentWeights[1]
    assert swappool._targetWeight(token1) == 20
    assert swappool._targetWeight(token2) == 30

    chain.snapshot()

    chain.mine(1, timestamp=int(startTime + ONEWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = swappool._lastModificationTime()
    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    tx = swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    passedTime = (tx.timestamp - lastModification)/(duration)

    assert swappool._weight(token1) == currentWeights[0] * (1 - passedTime) + targetWeight[0] * passedTime
    assert swappool._weight(token2) == currentWeights[1] * (1 - passedTime) + targetWeight[1] * passedTime
    lastWeights = [swappool._weight(token1),  swappool._weight(token2)]

    chain.mine(1, timestamp=int(startTime + ONEWEEK + 5))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._weight(token1) == targetWeight[0]
    assert swappool._weight(token2) == targetWeight[1]


def test_decrease_weights(gov, default_amp_swappool, token1, token2, chain):
    swappool = default_amp_swappool


    startTime = chain.time()
    swappool.modifyWeights(startTime + ONEWEEK, [2, 300, 500], {"from": gov})
    chain.mine(1, timestamp=int(startTime + ONEWEEK))
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    currentWeights = [swappool._weight(tkn) for tkn in [token1, token2]]
    swappool.modifyWeights(startTime + ONEWEEK * 2, [2, 100, 100], {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()
    
    targetWeight = [swappool._targetWeight(tkn) for tkn in [token1, token2]]

    assert swappool._targetWeight(token1) == 2
    assert swappool._targetWeight(token2) == 100

    chain.mine(1, timestamp=int(startTime + ONEWEEK + ONEWEEK / 3))
    
    lastModification = swappool._lastModificationTime()
    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    tx = swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    passedTime = (tx.timestamp - lastModification)/(duration)

    assert swappool._weight(token1) == 2
    assert swappool._weight(token2) == ceil(currentWeights[1] * (1 - passedTime) + targetWeight[1] * (passedTime))

    chain.mine(1, timestamp=int(startTime + ONEWEEK * 2 + 5))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._weight(token2) == 100