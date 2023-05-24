import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWOWEEK = ONEWEEK * 2

# Create a 'local_vault' fixture to replace the 'vault' fixture, as the vaults of the
# latter are always 'cross chain' (i.e. they have a non-zero interface address)
@pytest.fixture(scope="module")
def local_vault(group_config, group_tokens, deploy_vault, vault_index, deployer):
    yield deploy_vault(
        tokens         = group_tokens[vault_index],
        token_balances = group_config[vault_index]["init_balances"],
        weights        = group_config[vault_index]["weights"],
        amp            = group_config[vault_index]["amplification"],
        name           = group_config[vault_index]["name"],
        symbol         = group_config[vault_index]["symbol"],
        deployer       = deployer,
        only_local     = True   # ! important
    )

@pytest.mark.no_call_coverage
def test_increase_amp(local_vault, math_lib_amp, vault_tokens, deployer):
        
    currAmp = 10**18 - local_vault._oneMinusAmp()
    
    startTime = chain.time()
    targetAmp = min(int(currAmp * 1.9), 10**18-1)
    assert targetAmp > currAmp
    local_vault.setAmplification(startTime + TWOWEEK, targetAmp, {"from": deployer})
    duration = local_vault._adjustmentTarget() - local_vault._lastModificationTime()
    
    # Amplification should not change immediately.
    assert 10**18 - local_vault._oneMinusAmp() == currAmp
    assert 10**18 - local_vault._targetAmplification() == targetAmp
    
    # Weights should be updated automatically after a block has been mined.
    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))
    
    lastModification = local_vault._lastModificationTime()
    passedTime = (chain[-1].timestamp - lastModification)/(duration)
    
    # Check that it truely updated.
    assert (10**18 - math_lib_amp.getTrueAmp(local_vault))//1000 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//1000
    
    # Check that the on-chain variable has not changed.
    assert 10**18 - local_vault._oneMinusAmp() < 10**18 - math_lib_amp.getTrueAmp(local_vault)
    assert 10**18 - local_vault._targetAmplification() > 10**18 - math_lib_amp.getTrueAmp(local_vault)

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    assert (10**18 - math_lib_amp.getTrueAmp(local_vault)) == targetAmp


@pytest.mark.no_call_coverage
def test_decrease_amp(local_vault, math_lib_amp, vault_tokens, deployer):
    
    # Decrease the amplification.
    startTime = chain.time()
    currAmp = 10**18 - local_vault._oneMinusAmp()
    targetAmp = int(currAmp / 1.9)
    local_vault.setAmplification(startTime + TWOWEEK, targetAmp, {"from": deployer})
    duration = local_vault._adjustmentTarget() - local_vault._lastModificationTime()

    # Amplification should not change immediately.
    assert (10**18 - local_vault._oneMinusAmp()) == currAmp
    assert (10**18 - local_vault._targetAmplification()) == targetAmp

    # Weights should be updated automatically after a block has been mined.
    chain.mine(1, timestamp=int(startTime + TWOWEEK / 3))
    
    # Sadly the amplification are not updated automatically, we can call swap to update though.
    lastModification = local_vault._lastModificationTime()
    passedTime = (chain[-1].timestamp - lastModification)/(duration)

    # Check that it truely updated.
    assert (10**18 - math_lib_amp.getTrueAmp(local_vault))//1000 == floor(currAmp * (1 - passedTime) + targetAmp * passedTime)//1000
    
    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    assert (10**18 - math_lib_amp.getTrueAmp(local_vault)) == targetAmp

