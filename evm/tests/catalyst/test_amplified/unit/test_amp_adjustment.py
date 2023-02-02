import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWOWEEK = ONEWEEK * 2


def test_only_administrator(pool, deployer, berg):
    startTime = chain.time()
    with reverts():
        pool.modifyAmplification(startTime + TWOWEEK, 10**15, {"from": berg})

    pool.modifyAmplification(startTime + TWOWEEK, 10**15, {"from": deployer})


def test_1_week_minimum(pool, deployer):
    with reverts():
        pool.modifyAmplification(chain.time() + ONEWEEK - 1, 10**15, {"from": deployer})

    pool.modifyAmplification(chain.time() + ONEWEEK + 1, 10**15, {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_amp(pool, pool_tokens, deployer):
    currAmp = pool._amp()
    
    startTime = chain.time()
    targetAmp = 8 * 10**17
    assert targetAmp > currAmp
    pool.modifyAmplification(startTime + TWOWEEK, targetAmp, {"from": deployer})
    pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = pool._adjustmentTarget() - pool._lastModificationTime()
    
    # Weights should not change immediately.
    assert pool._amp() == currAmp
    assert pool._targetAmplification() == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = pool._lastModificationTime()
    tx = pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    # Be mostly accurate.
    assert pool._amp()//10 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//10

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    assert pool._amp() == targetAmp


@pytest.mark.no_call_coverage
def test_decrease_amp(pool, pool_tokens, deployer):
    startTime = chain.time()
    # Increase the weights
    currAmp = 10**17
    pool.modifyAmplification(startTime + TWOWEEK, currAmp, {"from": deployer})
    chain.mine(1, timestamp=int(startTime + TWOWEEK + 10))
    pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    
    # Decrease the weights.
    targetAmp = 10**15
    pool.modifyAmplification(startTime + TWOWEEK * 2, targetAmp, {"from": deployer})
    pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = pool._adjustmentTarget() - pool._lastModificationTime()

    # Weights should not change immediately.
    assert pool._amp() == currAmp
    assert pool._targetAmplification() == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK + TWOWEEK / 3))
    
    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = pool._lastModificationTime()
    tx = pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    # Be mostly accurate.
    assert pool._amp()//10 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//10
    
    chain.mine(1, timestamp=int(startTime + TWOWEEK * 2 + 100))

    pool.localswap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    assert pool._amp() == targetAmp

