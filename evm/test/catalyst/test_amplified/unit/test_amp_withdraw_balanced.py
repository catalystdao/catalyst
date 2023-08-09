import pytest
from brownie import chain
from brownie.test import given, strategy
from hypothesis import example


# This function compares the output difference between withdrawAll and withdrawMixed
@example(percentage=3000)
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
@pytest.mark.no_call_coverage
def test_withdrawall(vault, vault_tokens, berg, deployer, percentage):
    percentage /= 10000

    vaultBalances = [token.balanceOf(vault) for token in vault_tokens]
    vaultTokens = int(vault.balanceOf(deployer) * percentage)
    vault.transfer(berg, vaultTokens, {"from": deployer})
    ts = vault.totalSupply()

    tx_all = vault.withdrawAll(vaultTokens, [0 for _ in vault_tokens], {"from": berg})

    withdrawAllAmount = tx_all.return_value

    for allAmount, vaultBalance in zip(withdrawAllAmount, vaultBalances):
        if allAmount > vaultBalance * vaultTokens // ts:
            assert allAmount <= int(
                vaultBalance * vaultTokens // ts * (1 + 1e-10) + 1
            )  # If more is returned, it needs to be almost insignificant. (and there is a deposit fee.)
        assert int(vaultBalance * percentage * 99 / 100) <= allAmount


# This function compares the output difference between withdrawAll and withdrawMixed
@example(percentage=3000)
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
@pytest.mark.no_call_coverage
def test_compare_withdrawall_and_withdrawmixed(
    vault, vault_tokens, berg, deployer, percentage
):
    percentage /= 10000

    vaultTokens = int(vault.balanceOf(deployer) * percentage)
    vault.transfer(berg, vaultTokens, {"from": deployer})

    tx_all = vault.withdrawAll(vaultTokens, [0 for _ in vault_tokens], {"from": berg})

    withdrawAllAmount = tx_all.return_value
    chain.undo()

    tx_mixed = vault.withdrawMixed(
        vaultTokens,
        [int(10**18 / (len(vault_tokens) - i)) for i in range(len(vault_tokens))],
        [0 for _ in vault_tokens],
        {"from": berg},
    )

    withdrawMixedAmount = tx_mixed.return_value

    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        if mixedAmount > allAmount:
            assert mixedAmount <= int(
                allAmount * (1 + 1e-10) + 1
            )  # If more is returned, it needs to be almost insignificant. (and there is a deposit fee.)
        assert int(allAmount * 99 / 100) <= mixedAmount
