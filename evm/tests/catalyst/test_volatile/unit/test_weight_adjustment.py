import pytest
from brownie import ZERO_ADDRESS, chain, reverts
from brownie.test import given, strategy
from math import ceil, floor

ONEWEEK = 60 * 60 * 24 * 7
TWOWEEK = ONEWEEK * 2


@pytest.fixture(scope="module")
def set_minimum_weights(vault, vault_tokens, deployer):
    def _set_minimum_weights(min_desired_weights):
        current_weights = [vault._weight(tkn) for tkn in vault_tokens]
        min_desired_weights = min_desired_weights[: len(current_weights)]

        while not all(
            [c_w >= m_w for c_w, m_w in zip(current_weights, min_desired_weights)]
        ):
            adjustment_time = chain.time() + ONEWEEK + 1
            next_weights = [
                min(c_weight * 10, d_weight)
                for c_weight, d_weight in zip(current_weights, min_desired_weights)
            ]

            vault.setWeights(adjustment_time, next_weights, {"from": deployer})
            chain.mine(1, timestamp=int(adjustment_time + 1))
            vault.localSwap(
                vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer}
            )  # Trigger weights update

            current_weights = [vault._weight(tkn) for tkn in vault_tokens]

    yield _set_minimum_weights


@pytest.mark.no_vault_param
def test_only_administrator(vault, deployer, berg):
    startTime = chain.time()
    with reverts():
        vault.setWeights(startTime + TWOWEEK, [2, 3, 5], {"from": berg})

    vault.setWeights(startTime + TWOWEEK, [2, 3, 5], {"from": deployer})


@pytest.mark.no_vault_param
def test_1_week_minimum(vault, deployer):
    with reverts():
        vault.setWeights(chain.time() + ONEWEEK - 1, [2, 3, 5], {"from": deployer})

    vault.setWeights(chain.time() + ONEWEEK + 1, [2, 3, 5], {"from": deployer})


@pytest.mark.no_vault_param
def test_max_weights_increase(vault, vault_tokens, deployer):

    currentWeights = [vault._weight(tkn) for tkn in vault_tokens]

    tooLargeWeights = [weight * 11 for weight in currentWeights]
    with reverts():
        vault.setWeights(
            chain.time() + ONEWEEK + 1, tooLargeWeights, {"from": deployer}
        )

    maxWeights = [weight * 10 for weight in currentWeights]
    vault.setWeights(chain.time() + ONEWEEK + 1, maxWeights, {"from": deployer})


@pytest.mark.no_vault_param
def test_max_weights_decrease(vault, vault_tokens, deployer, set_minimum_weights):

    set_minimum_weights([100, 100, 100])

    currentWeights = [vault._weight(tkn) for tkn in vault_tokens]

    tooSmallWeights = [weight / 11 for weight in currentWeights]
    with reverts():
        vault.setWeights(
            chain.time() + ONEWEEK + 1, tooSmallWeights, {"from": deployer}
        )

    minWeights = [weight / 10 for weight in currentWeights]
    vault.setWeights(chain.time() + ONEWEEK + 1, minWeights, {"from": deployer})


@pytest.mark.no_call_coverage
def test_increase_weights(vault, vault_tokens, deployer, set_minimum_weights):

    # Make sure the weights are at least in the 'tens' range to better measure the weights update
    set_minimum_weights([10, 10, 10])

    startTime = chain.time()
    currentWeights = [vault._weight(tkn) for tkn in vault_tokens]
    increaseFactors = [2, 3, 5]  # Note factors must be less than or equal to 10 each
    targetWeights = [
        weight * factor for weight, factor in zip(currentWeights, increaseFactors)
    ]

    vault.setWeights(startTime + TWOWEEK, targetWeights, {"from": deployer})
    duration = vault._adjustmentTarget() - vault._lastModificationTime()

    targetWeight = [vault._targetWeight(tkn) for tkn in vault_tokens]

    # Weights should not change immediately.
    for token, currWeight, targetWeight in zip(
        vault_tokens, currentWeights, targetWeights
    ):
        assert vault._weight(token) == currWeight
        assert vault._targetWeight(token) == targetWeight

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 2))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = vault._lastModificationTime()
    tx = vault.localSwap(vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification) / (duration)

    for token, currWeight, targetWeight in zip(
        vault_tokens, currentWeights, targetWeights
    ):
        assert vault._weight(token) == floor(
            currWeight * (1 - passedTime) + targetWeight * passedTime
        )

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    vault.localSwap(vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer})

    for token, targetWeight in zip(vault_tokens, targetWeights):
        assert vault._weight(token) == targetWeight


@pytest.mark.no_call_coverage
def test_decrease_weights(vault, vault_tokens, deployer, set_minimum_weights):
    # Increase the weights
    set_minimum_weights([2, 300, 500])

    # Decrease the weights.
    startTime = chain.time()
    currentWeights = [vault._weight(tkn) for tkn in vault_tokens]
    decreaseFactors = [1, 3, 5]  # Note factors must be less than or equal to 10 each
    targetWeights = [
        weight / factor for weight, factor in zip(currentWeights, decreaseFactors)
    ]
    vault.setWeights(startTime + TWOWEEK, targetWeights, {"from": deployer})
    duration = vault._adjustmentTarget() - vault._lastModificationTime()

    targetWeight = [vault._targetWeight(tkn) for tkn in vault_tokens]

    # Weights should not change immediately.
    for token, currWeight, targetWeight in zip(
        vault_tokens, currentWeights, targetWeights
    ):
        assert vault._weight(token) == currWeight
        assert vault._targetWeight(token) == targetWeight

    chain.mine(1, timestamp=int(startTime + TWOWEEK / 3))

    # Sadly the weights are not updated automatically, we can call swap to update though.
    lastModification = vault._lastModificationTime()
    tx = vault.localSwap(vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer})
    passedTime = (tx.timestamp - lastModification) / (duration)

    for token, currWeight, targetWeight in zip(
        vault_tokens, currentWeights, targetWeights
    ):
        assert vault._weight(token) == ceil(
            currWeight * (1 - passedTime) + targetWeight * passedTime
        )

    chain.mine(1, timestamp=int(startTime + TWOWEEK))

    vault.localSwap(vault_tokens[0], vault_tokens[0], 0, 0, {"from": deployer})

    for token, targetWeight in zip(vault_tokens, targetWeights):
        assert vault._weight(token) == targetWeight
