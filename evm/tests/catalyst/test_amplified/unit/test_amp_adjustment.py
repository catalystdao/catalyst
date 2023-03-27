import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWOWEEK = ONEWEEK * 2

# Create a 'local_pool' fixture to replace the 'pool' fixture, as the pools of the
# latter are always 'cross chain' (i.e. they have a non-zero interface address)
@pytest.fixture(scope="module")
def local_pool(group_config, group_tokens, deploy_pool, pool_index, deployer):
    yield deploy_pool(
        tokens         = group_tokens[pool_index],
        token_balances = group_config[pool_index]["init_balances"],
        weights        = group_config[pool_index]["weights"],
        amp            = group_config[pool_index]["amplification"],
        name           = group_config[pool_index]["name"],
        symbol         = group_config[pool_index]["symbol"],
        deployer       = deployer,
        only_local     = True   # ! important
    )


def test_amp_cross_chain_pools_disabled(pool, deployer, amplification):
    if pool._chainInterface() != ZERO_ADDRESS:
        with reverts("dev: Amplification adjustment is disabled for cross-chain pools."):
            pool.setAmplification(chain.time() + TWOWEEK, amplification, {"from": deployer})


@pytest.mark.no_pool_param
def test_only_administrator(local_pool, deployer, berg, amplification):

    with reverts():
        local_pool.setAmplification(chain.time() + TWOWEEK, amplification, {"from": berg})

    local_pool.setAmplification(chain.time() + TWOWEEK, amplification, {"from": deployer})


@pytest.mark.no_pool_param
def test_1_week_minimum(local_pool, deployer, amplification):
        
    with reverts():
        local_pool.setAmplification(chain.time() + ONEWEEK - 1, amplification, {"from": deployer})

    local_pool.setAmplification(chain.time() + ONEWEEK + 1, amplification, {"from": deployer})


@pytest.mark.no_pool_param
def test_max_amp_increase(local_pool, deployer):

    currentAmp = 10**18 - local_pool._oneMinusAmp()

    tooLargeAmp = int(currentAmp * 2.1)
    with reverts():
        local_pool.setAmplification(chain.time() + ONEWEEK + 1, tooLargeAmp, {"from": deployer})

    maxAmp = min(int(currentAmp * 2), 10**18-1)
    local_pool.setAmplification(chain.time() + ONEWEEK + 1, maxAmp, {"from": deployer})


@pytest.mark.no_pool_param
def test_max_amp_decrease(local_pool, deployer):

    currentAmp = 10**18 - local_pool._oneMinusAmp()

    tooSmallAmp = int(currentAmp / 2.1)
    with reverts():
        local_pool.setAmplification(chain.time() + ONEWEEK + 1, tooSmallAmp, {"from": deployer})

    minAmp = int(currentAmp / 2)
    local_pool.setAmplification(chain.time() + ONEWEEK + 1, minAmp, {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_amp(local_pool, pool_tokens, deployer):
        
    currAmp = 10**18 - local_pool._oneMinusAmp()
    
    startTime = chain.time()
    targetAmp = min(int(currAmp * 1.9), 10**18-1)
    assert targetAmp > currAmp
    local_pool.setAmplification(startTime + TWOWEEK, targetAmp, {"from": deployer})
    local_pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = local_pool._adjustmentTarget() - local_pool._lastModificationTime()
    
    # Weights should not change immediately.
    assert 10**18 - local_pool._oneMinusAmp() == currAmp
    assert 10**18 - local_pool._targetAmplification() == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = local_pool._lastModificationTime()
    tx = local_pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    # Be mostly accurate.
    assert (10**18 - local_pool._oneMinusAmp())//1000 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//1000

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    local_pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    assert (10**18 - local_pool._oneMinusAmp()) == targetAmp


@pytest.mark.no_call_coverage
def test_decrease_amp(local_pool, pool_tokens, deployer):
    
    # Decrease the amplification.
    startTime = chain.time()
    currAmp = 10**18 - local_pool._oneMinusAmp()
    targetAmp = int(currAmp / 1.9)
    local_pool.setAmplification(startTime + TWOWEEK + 1, targetAmp, {"from": deployer})
    local_pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    duration = local_pool._adjustmentTarget() - local_pool._lastModificationTime()

    # Amplification should not change immediately.
    assert (10**18 - local_pool._oneMinusAmp()) == currAmp
    assert (10**18 - local_pool._targetAmplification()) == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 3))
    
    # Sadly the amplification are not updated automatically, we can call swap to update though.
    lastModification = local_pool._lastModificationTime()
    tx = local_pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification)/(duration)

    # Be mostly accurate.
    assert (10**18 - local_pool._oneMinusAmp())//1000 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//1000
    
    chain.mine(1, timestamp=int(startTime + TWOWEEK + 100))

    local_pool.localSwap(pool_tokens[0], pool_tokens[0], 0, 0, {"from": deployer})

    assert (10**18 - local_pool._oneMinusAmp()) == targetAmp

