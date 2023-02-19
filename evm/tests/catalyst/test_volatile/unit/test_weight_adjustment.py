import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWOWEEK = ONEWEEK * 2


@pytest.mark.no_pool_param
def test_only_administrator(pool, deployer, berg):
    startTime = chain.time()
    with reverts():
        pool.modifyWeights(startTime + TWOWEEK, [2, 3, 5], {"from": berg})

    pool.modifyWeights(startTime + TWOWEEK, [2, 3, 5], {"from": deployer})


@pytest.mark.no_pool_param
def test_1_week_minimum(pool, deployer):
    with reverts():
        pool.modifyWeights(chain.time() + ONEWEEK - 1, [2, 3, 5], {"from": deployer})

    pool.modifyWeights(chain.time() + ONEWEEK + 1, [2, 3, 5], {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_weights(pool, pool_tokens, deployer):
    currentWeights = [pool._weight(tkn) for tkn in pool_tokens]

    startTime = chain.time()
    targetWeights = [20, 30, 50]
    pool.modifyWeights(startTime + TWOWEEK, [20, 30, 50], {"from": deployer})
    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = pool._adjustmentTarget() - pool._lastModificationTime()
    
    targetWeight = [pool._targetWeight(tkn) for tkn in pool_tokens]

    # Weights should not change immediately.
    for token, currWeight, targetWeight in zip(pool_tokens, currentWeights, targetWeights):
        assert pool._weight(token) == currWeight
        assert pool._targetWeight(token) == targetWeight

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = pool._lastModificationTime()
    tx = pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    for token, currWeight, targetWeight in zip(pool_tokens, currentWeights, targetWeights):
        assert pool._weight(token) == floor(currWeight * (1 - passedTime) + targetWeight * passedTime)

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    for token, targetWeight in zip(pool_tokens, targetWeights):
        assert pool._weight(token) == targetWeight


@pytest.mark.no_call_coverage
def test_decrease_weights(pool, pool_tokens, deployer):
    startTime = chain.time()
    # Increase the weights
    pool.modifyWeights(startTime + TWOWEEK, [2, 300, 500], {"from": deployer})
    chain.mine(1, timestamp=int(startTime + TWOWEEK))
    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    currentWeights = [pool._weight(tkn) for tkn in pool_tokens]
    
    # Decrease the weights.
    targetWeights = [2, 100, 100]
    pool.modifyWeights(startTime + TWOWEEK * 2, targetWeights, {"from": deployer})
    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = pool._adjustmentTarget() - pool._lastModificationTime()
    
    targetWeight = [pool._targetWeight(tkn) for tkn in pool_tokens]

    # Weights should not change immediately.
    for token, currWeight, targetWeight in zip(pool_tokens, currentWeights, targetWeights):
        assert pool._weight(token) == currWeight
        assert pool._targetWeight(token) == targetWeight

    chain.mine(1, timestamp=int(startTime + TWOWEEK + TWOWEEK / 3))
    
    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = pool._lastModificationTime()
    tx = pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)


    for token, currWeight, targetWeight in zip(pool_tokens, currentWeights, targetWeights):
        assert pool._weight(token) == ceil(currWeight * (1 - passedTime) + targetWeight * passedTime)
    
    chain.mine(1, timestamp=int(startTime + TWOWEEK * 2))

    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    for token, targetWeight in zip(pool_tokens, targetWeights):
        assert pool._weight(token) == targetWeight

