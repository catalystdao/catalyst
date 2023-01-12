import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWODAYS = 60 * 60 * 24 * 2


def test_only_administrator(swappool, deployer, berg):
    startTime = chain.time()
    with reverts():
        swappool.modifyWeights(startTime + ONEWEEK, [2, 3, 5], {"from": berg})

    swappool.modifyWeights(startTime + ONEWEEK, [2, 3, 5], {"from": deployer})


def test_2_days_minimum(swappool, deployer):
    with reverts():
        swappool.modifyWeights(chain.time() + TWODAYS - 1, [2, 3, 5], {"from": deployer})

    swappool.modifyWeights(chain.time() + TWODAYS + 1, [2, 3, 5], {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_weights(swappool, deployer, get_pool_tokens):
    tokens = get_pool_tokens(swappool)
    currentWeights = [swappool._weight(tkn) for tkn in tokens]

    startTime = chain.time()
    targetWeights = [20, 30, 50]
    swappool.modifyWeights(startTime + ONEWEEK, [20, 30, 50], {"from": deployer})
    swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()
    
    targetWeight = [swappool._targetWeight(tkn) for tkn in tokens]

    # Weights should not change immediately.
    for token, currWeight, targetWeight in zip(tokens, currentWeights, targetWeights):
        assert swappool._weight(token) == currWeight
        assert swappool._targetWeight(token) == targetWeight

    chain.mine(1, timestamp=int(startTime + ONEWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = swappool._lastModificationTime()
    tx = swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    for token, currWeight, targetWeight in zip(tokens, currentWeights, targetWeights):
        assert swappool._weight(token) == floor(currWeight * (1 - passedTime) + targetWeight * passedTime)

    chain.mine(1, timestamp=int(startTime + ONEWEEK))

    swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})

    for token, targetWeight in zip(tokens, targetWeights):
        assert swappool._weight(token) == targetWeight


@pytest.mark.no_call_coverage
def test_decrease_weights(swappool, deployer, get_pool_tokens):
    tokens = get_pool_tokens(swappool)
    
    startTime = chain.time()
    # Increase the weights
    swappool.modifyWeights(startTime + ONEWEEK, [2, 300, 500], {"from": deployer})
    chain.mine(1, timestamp=int(startTime + ONEWEEK))
    swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})
    currentWeights = [swappool._weight(tkn) for tkn in tokens]
    
    # Decrease the weights.
    targetWeights = [2, 100, 100]
    swappool.modifyWeights(startTime + ONEWEEK * 2, targetWeights, {"from": deployer})
    swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})
    duration = swappool._adjustmentTarget() - swappool._lastModificationTime()
    
    targetWeight = [swappool._targetWeight(tkn) for tkn in tokens]

    # Weights should not change immediately.
    for token, currWeight, targetWeight in zip(tokens, currentWeights, targetWeights):
        assert swappool._weight(token) == currWeight
        assert swappool._targetWeight(token) == targetWeight

    chain.mine(1, timestamp=int(startTime + ONEWEEK + ONEWEEK / 3))
    
    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = swappool._lastModificationTime()
    tx = swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)


    for token, currWeight, targetWeight in zip(tokens, currentWeights, targetWeights):
        assert swappool._weight(token) == ceil(currWeight * (1 - passedTime) + targetWeight * passedTime)
    
    chain.mine(1, timestamp=int(startTime + ONEWEEK * 2))

    swappool.localswap(tokens[0], tokens[1], 0, 0, False, {"from": deployer})

    for token, targetWeight in zip(tokens, targetWeights):
        assert swappool._weight(token) == targetWeight

