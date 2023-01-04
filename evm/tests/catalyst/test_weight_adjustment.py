import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain
from brownie.test import given, strategy
from hypothesis import settings
from math import ceil, floor


@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


ONEWEEK = 60 * 60 * 24 * 7
TWODAYS = 60 * 60 * 24 * 2


def test_only_administrator(gov, accounts, default_swappool):
    swappool = default_swappool

    startTime = chain.time()
    with brownie.reverts():
        swappool.modifyWeights(startTime + ONEWEEK, [2, 3, 5], {"from": accounts[2]})

    swappool.modifyWeights(startTime + ONEWEEK, [2, 3, 5], {"from": gov})


def test_2_days_minimum(gov, default_swappool):
    swappool = default_swappool

    with brownie.reverts():
        swappool.modifyWeights(chain.time() + TWODAYS - 1, [2, 3, 5], {"from": gov})

    swappool.modifyWeights(chain.time() + TWODAYS + 1, [2, 3, 5], {"from": gov})


def test_increase_weights(gov, default_swappool, token1, token2, token3, chain):
    swappool = default_swappool

    currentWeights = [swappool._weight(tkn) for tkn in [token1, token2, token3]]

    startTime = chain.time()
    swappool.modifyWeights(startTime + ONEWEEK, [20, 30, 50], {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()
    
    targetWeight = [swappool._targetWeight(tkn) for tkn in [token1, token2, token3]]

    # Weights should not change immediately.
    assert swappool._weight(token1) == currentWeights[0]
    assert swappool._weight(token2) == currentWeights[1]
    assert swappool._weight(token3) == currentWeights[2]
    assert swappool._targetWeight(token1) == 20
    assert swappool._targetWeight(token2) == 30
    assert swappool._targetWeight(token3) == 50

    chain.snapshot()

    chain.mine(1, timestamp=int(startTime + ONEWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.

    assert swappool._weight(token1) == currentWeights[0]
    assert swappool._weight(token2) == currentWeights[1]
    assert swappool._weight(token3) == currentWeights[2]
    
    lastModification = swappool._lastModificationTime()
    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    tx = swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    passedTime = (tx.timestamp - lastModification)/(duration)

    assert swappool._weight(token1) == currentWeights[0] * (1 - passedTime) + targetWeight[0] * passedTime
    assert swappool._weight(token2) == currentWeights[1] * (1 - passedTime) + targetWeight[1] * passedTime
    assert swappool._weight(token3) == currentWeights[2] * (1 - passedTime) + targetWeight[2] * passedTime

    chain.mine(1, timestamp=int(startTime + ONEWEEK))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._weight(token1) == targetWeight[0]
    assert swappool._weight(token2) == targetWeight[1]
    assert swappool._weight(token3) == targetWeight[2]


def test_decrease_weights(gov, default_swappool, token1, token2, token3, chain):
    swappool = default_swappool


    startTime = chain.time()
    swappool.modifyWeights(startTime + ONEWEEK, [2, 300, 500], {"from": gov})
    chain.mine(1, timestamp=int(startTime + ONEWEEK))
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    currentWeights = [swappool._weight(tkn) for tkn in [token1, token2, token3]]
    swappool.modifyWeights(startTime + ONEWEEK * 2, [2, 100, 100], {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()
    
    targetWeight = [swappool._targetWeight(tkn) for tkn in [token1, token2, token3]]

    # Weights should not change immediately.
    assert swappool._weight(token1) == currentWeights[0]
    assert swappool._weight(token2) == currentWeights[1]
    assert swappool._weight(token3) == currentWeights[2]
    assert swappool._targetWeight(token1) == 2
    assert swappool._targetWeight(token2) == 100
    assert swappool._targetWeight(token3) == 100

    chain.snapshot()

    chain.mine(1, timestamp=int(startTime + ONEWEEK + ONEWEEK / 3))
    
    lastModification = swappool._lastModificationTime()
    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    tx = swappool.localswap(token1, token2, 0, 0, False, {"from": gov})
    passedTime = (tx.timestamp - lastModification)/(duration)

    assert swappool._weight(token1) == 2
    assert swappool._weight(token2) == ceil(currentWeights[1] * (1 - passedTime) + targetWeight[1] * (passedTime))
    assert swappool._weight(token3) == ceil(currentWeights[2] * (1 - passedTime) + targetWeight[2] * (passedTime))
    
    chain.mine(1, timestamp=int(startTime + ONEWEEK * 2))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._weight(token2) == 100
    assert swappool._weight(token3) == 100


def test_increase_weights_2_tokens(gov, default_swappool_2, token1, token2, chain):
    swappool = default_swappool_2

    currentWeights = [swappool._weight(tkn) for tkn in [token1, token2]]

    startTime = chain.time()
    swappool.modifyWeights(startTime + ONEWEEK, [700, 300], {"from": gov})
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    targetWeight = [swappool._targetWeight(tkn) for tkn in [token1, token2]]

    # Weights should not change immediately.
    assert swappool._weight(token1) == currentWeights[0]
    assert swappool._weight(token2) == currentWeights[1]
    assert swappool._targetWeight(token1) == 700
    assert swappool._targetWeight(token2) == 300

    chain.snapshot()

    chain.mine(1, timestamp=int(startTime + ONEWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.

    assert swappool._weight(token1) == currentWeights[0]
    assert swappool._weight(token2) == currentWeights[1]

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._weight(token1) == (currentWeights[0] + targetWeight[0]) / 2
    assert swappool._weight(token2) == (currentWeights[1] + targetWeight[1]) / 2

    chain.mine(1, timestamp=int(startTime + ONEWEEK))

    # token1.approve(swappool, 2**256-1, {'from': gov}) # not needed, since 0.
    swappool.localswap(token1, token2, 0, 0, False, {"from": gov})

    assert swappool._weight(token1) == targetWeight[0]
    assert swappool._weight(token2) == targetWeight[1]
