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
        tokens=group_tokens[vault_index],
        token_balances=group_config[vault_index]["init_balances"],
        weights=group_config[vault_index]["weights"],
        amp=group_config[vault_index]["amplification"],
        name=group_config[vault_index]["name"],
        symbol=group_config[vault_index]["symbol"],
        deployer=deployer,
        only_local=True,  # ! important
    )


def test_amp_cross_chain_vaults_disabled(vault, deployer, amplification):
    if vault._chainInterface() != ZERO_ADDRESS:
        with reverts():  # "dev: Amplification adjustment is disabled for cross-chain vaults."
            vault.setAmplification(
                chain.time() + TWOWEEK, amplification, {"from": deployer}
            )


@pytest.mark.no_vault_param
def test_only_administrator(local_vault, deployer, berg, amplification):

    with reverts():
        local_vault.setAmplification(
            chain.time() + TWOWEEK, amplification, {"from": berg}
        )

    local_vault.setAmplification(
        chain.time() + TWOWEEK, amplification, {"from": deployer}
    )


@pytest.mark.no_vault_param
def test_1_week_minimum(local_vault, deployer, amplification):

    with reverts():
        local_vault.setAmplification(
            chain.time() + ONEWEEK - 1, amplification, {"from": deployer}
        )

    local_vault.setAmplification(
        chain.time() + ONEWEEK + 1, amplification, {"from": deployer}
    )


@pytest.mark.no_vault_param
def test_max_amp_increase(local_vault, deployer):

    currentAmp = 10**18 - local_vault._oneMinusAmp()

    tooLargeAmp = int(currentAmp * 2.1)
    with reverts():
        local_vault.setAmplification(
            chain.time() + ONEWEEK + 1, tooLargeAmp, {"from": deployer}
        )

    maxAmp = min(int(currentAmp * 2), 10**18 - 1)
    local_vault.setAmplification(chain.time() + ONEWEEK + 1, maxAmp, {"from": deployer})


@pytest.mark.no_vault_param
def test_max_amp_decrease(local_vault, deployer):

    currentAmp = 10**18 - local_vault._oneMinusAmp()

    tooSmallAmp = int(currentAmp / 2.1)
    with reverts():
        local_vault.setAmplification(
            chain.time() + ONEWEEK + 1, tooSmallAmp, {"from": deployer}
        )

    minAmp = int(currentAmp / 2)
    local_vault.setAmplification(chain.time() + ONEWEEK + 1, minAmp, {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_amp(local_vault, vault_tokens, deployer):

    currAmp = 10**18 - local_vault._oneMinusAmp()

    startTime = chain.time()
    targetAmp = min(int(currAmp * 1.9), 10**18 - 1)
    assert targetAmp > currAmp
    local_vault.setAmplification(startTime + TWOWEEK, targetAmp, {"from": deployer})
    duration = local_vault._adjustmentTarget() - local_vault._lastModificationTime()

    # Amplification should not change immediately.
    assert 10**18 - local_vault._oneMinusAmp() == currAmp
    assert 10**18 - local_vault._targetAmplification() == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = local_vault._lastModificationTime()
    tx = local_vault.localSwap(
        vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer}
    )
    passedTime = (tx.timestamp - lastModification) / (duration)

    # Be mostly accurate.
    assert (10**18 - local_vault._oneMinusAmp()) // 1000 == floor(
        currAmp * (1 - passedTime) + targetAmp * passedTime
    ) // 1000

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    local_vault.localSwap(vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer})

    assert (10**18 - local_vault._oneMinusAmp()) == targetAmp


@pytest.mark.no_call_coverage
def test_decrease_amp(local_vault, vault_tokens, deployer):

    # Decrease the amplification.
    startTime = chain.time()
    currAmp = 10**18 - local_vault._oneMinusAmp()
    targetAmp = int(currAmp / 1.9)
    local_vault.setAmplification(startTime + TWOWEEK + 1, targetAmp, {"from": deployer})
    duration = local_vault._adjustmentTarget() - local_vault._lastModificationTime()

    # Amplification should not change immediately.
    assert (10**18 - local_vault._oneMinusAmp()) == currAmp
    assert (10**18 - local_vault._targetAmplification()) == targetAmp

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 3))

    # Sadly the amplification are not updated automatically, we can call swap to update though.
    lastModification = local_vault._lastModificationTime()
    tx = local_vault.localSwap(
        vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer}
    )
    passedTime = (tx.timestamp - lastModification) / (duration)

    # Be mostly accurate.
    assert (10**18 - local_vault._oneMinusAmp()) // 1000 == floor(
        currAmp * (1 - passedTime) + targetAmp * passedTime
    ) // 1000

    chain.mine(1, timestamp=int(startTime + TWOWEEK + 100))

    local_vault.localSwap(vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer})

    assert (10**18 - local_vault._oneMinusAmp()) == targetAmp
