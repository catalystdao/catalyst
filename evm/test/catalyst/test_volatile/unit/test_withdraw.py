import pytest
from brownie import chain
from brownie.test import given, strategy
from hypothesis import example


# This function compares the output difference between withdrawAll and withdrawMixed
@example(percentage=7000)
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
        assert allAmount <= vaultBalance * vaultTokens // ts
        assert int(vaultBalance * percentage * 9 / 10) <= allAmount


# This function compares the output difference between withdrawAll and withdrawMixed
@example(percentage=7000)
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
        # 0,00001% error is allowed on an upside. Any sane vault should implement a fee greater than this.
        # in which case the fee eats any potential upside.
        assert mixedAmount <= int(allAmount * (1 + 0.00001 / 2 / 100))

        assert int(allAmount * 7 / 10) <= mixedAmount
