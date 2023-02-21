import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWOWEEK = ONEWEEK * 2

@pytest.mark.no_pool_param
def test_only_administrator(pool, deployer, berg):
    if pool._chainInterface() != ZERO_ADDRESS:
        with reverts("dev: Amplification adjustment is disabled for cross-chain pools."):
            pool.setAmplification(chain.time() + TWOWEEK, 10**15, {"from": deployer})
        
        pytest.skip("Amplification adjustment is disabled for cross-chain pools")
        
    startTime = chain.time()
    with reverts():
        pool.setAmplification(startTime + TWOWEEK, 10**15, {"from": berg})

    pool.setAmplification(startTime + TWOWEEK, 10**15, {"from": deployer})


@pytest.mark.no_pool_param
def test_1_week_minimum(pool, deployer):
    if pool._chainInterface() != ZERO_ADDRESS:
        with reverts("dev: Amplification adjustment is disabled for cross-chain pools."):
            pool.setAmplification(chain.time() + TWOWEEK, 10**15, {"from": deployer})
            
        pytest.skip("Amplification adjustment is disabled for cross-chain pools")
        
    with reverts():
        pool.setAmplification(chain.time() + ONEWEEK - 1, 10**15, {"from": deployer})

    pool.setAmplification(chain.time() + ONEWEEK + 1, 10**15, {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_amp(pool, pool_tokens, deployer):
    if pool._chainInterface() != ZERO_ADDRESS:
        with reverts("dev: Amplification adjustment is disabled for cross-chain pools."):
            pool.setAmplification(chain.time() + TWOWEEK, 10**15, {"from": deployer})
        
        pytest.skip("Amplification adjustment is disabled for cross-chain pools")
        
    currAmp = 10**18 - pool._oneMinusAmp()
    
    startTime = chain.time()
    targetAmp = 8 * 10**17
    assert targetAmp > currAmp
    pool.setAmplification(startTime + TWOWEEK, targetAmp, {"from": deployer})
    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = pool._adjustmentTarget() - pool._lastModificationTime()
    
    # Weights should not change immediately.
    assert 10**18 - pool._oneMinusAmp() == currAmp
    assert pool._targetAmplification() == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = pool._lastModificationTime()
    tx = pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    # Be mostly accurate.
    assert (10**18 - pool._oneMinusAmp())//1000 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//1000

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    assert (10**18 - pool._oneMinusAmp()) == targetAmp


@pytest.mark.no_call_coverage
def test_decrease_amp(pool, pool_tokens, deployer):
    if pool._chainInterface() != ZERO_ADDRESS:
        with reverts("dev: Amplification adjustment is disabled for cross-chain pools."):
            pool.setAmplification(chain.time() + TWOWEEK, 10**15, {"from": deployer})
        
        pytest.skip("Amplification adjustment is disabled for cross-chain pools")
        
    startTime = chain.time()
    # Increase the weights
    currAmp = 10**17
    pool.setAmplification(startTime + TWOWEEK, currAmp, {"from": deployer})
    chain.mine(1, timestamp=int(startTime + TWOWEEK + 10))
    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    
    # Decrease the weights.
    targetAmp = 10**15
    pool.setAmplification(startTime + TWOWEEK * 2, targetAmp, {"from": deployer})
    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = pool._adjustmentTarget() - pool._lastModificationTime()

    # Weights should not change immediately.
    assert (10**18 - pool._oneMinusAmp()) == currAmp
    assert pool._targetAmplification() == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK + TWOWEEK / 3))
    
    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = pool._lastModificationTime()
    tx = pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    # Be mostly accurate.
    assert (10**18 - pool._oneMinusAmp())//1000 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//1000
    
    chain.mine(1, timestamp=int(startTime + TWOWEEK * 2 + 100))

    pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    assert (10**18 - pool._oneMinusAmp()) == targetAmp

