import pytest
from brownie import chain, reverts
from brownie.test import given, strategy
from hypothesis import example
from brownie.exceptions import VirtualMachineError
import tests.catalyst.utils.vault_utils as vault_utils


@example(unbalance_percentage=8000, withdrawal_percentage=9000)
@given(
    unbalance_percentage=strategy("uint256", min_value=100, max_value=10000),
    withdrawal_percentage=strategy("uint256", min_value=100, max_value=10000),
)
@pytest.mark.no_call_coverage
def test_withdrawall(
    vault,
    vault_tokens,
    get_vault_amp,
    berg,
    deployer,
    unbalance_percentage,
    withdrawal_percentage,
):
    if len(vault_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a vault to run a local swap.")

    unbalance_percentage /= 10000
    withdrawal_percentage /= 10000
    amp = get_vault_amp()

    vaultBalances = [token.balanceOf(vault) for token in vault_tokens]
    weights = [vault._weight(token) for token in vault_tokens]
    swap_amount = int(vaultBalances[0] * unbalance_percentage)
    vaultTokens = int(vault.totalSupply() * withdrawal_percentage)

    vault_tokens[0].transfer(berg, swap_amount, {"from": deployer})
    vault_tokens[0].approve(vault, swap_amount, {"from": berg})
    vault.transfer(berg, vaultTokens, {"from": deployer})

    tx_swap = vault.localSwap(
        vault_tokens[0], vault_tokens[1], swap_amount, 0, {"from": berg}
    )

    vaultBalances = [token.balanceOf(vault) for token in vault_tokens]

    tx_all = vault.withdrawAll(vaultTokens, [0 for _ in vault_tokens], {"from": berg})
    withdrawAllAmount = tx_all.return_value

    expectedAmounts = vault_utils.compute_equal_withdrawal(
        vaultTokens, weights, vaultBalances, vault.totalSupply() + vaultTokens, amp, 0
    )

    for allAmount, vaultBalance, expectedAmount in zip(
        withdrawAllAmount, vaultBalances, expectedAmounts
    ):
        if allAmount > expectedAmount:
            assert allAmount <= int(
                expectedAmount * (1 + 1e-10) + 1
            )  # If more is returned, it needs to be almost insignificant. (and there is a deposit fee.)
        assert int(expectedAmount * 99 / 100) <= allAmount


# This function compares the output difference between withdrawAll and withdrawMixed
@example(unbalance_percentage=8000, withdrawal_percentage=9000)
@given(
    unbalance_percentage=strategy("uint256", min_value=100, max_value=10000),
    withdrawal_percentage=strategy("uint256", min_value=100, max_value=10000),
)
@pytest.mark.no_call_coverage
def test_compare_withdrawall_and_withdrawmixed(
    vault, vault_tokens, berg, deployer, unbalance_percentage, withdrawal_percentage
):
    if len(vault_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a vault to run a local swap.")
    unbalance_percentage /= 10000
    withdrawal_percentage /= 10000

    vaultBalances = [token.balanceOf(vault) for token in vault_tokens]
    swap_amount = int(vaultBalances[0] * unbalance_percentage)
    vaultTokens = int(vault.totalSupply() * withdrawal_percentage)

    vault_tokens[0].transfer(berg, swap_amount, {"from": deployer})
    vault_tokens[0].approve(vault, swap_amount, {"from": berg})
    vault.transfer(berg, vaultTokens, {"from": deployer})

    tx_swap = vault.localSwap(
        vault_tokens[0], vault_tokens[1], swap_amount, 0, {"from": berg}
    )

    tx_all = vault.withdrawAll(vaultTokens, [0 for _ in vault_tokens], {"from": berg})

    withdrawAllAmount = tx_all.return_value
    all_swapvault_after_balances = [token.balanceOf(vault) for token in vault_tokens]
    chain.undo()

    try:
        tx_mixed = vault.withdrawMixed(
            vaultTokens,
            [int(10**18 / (len(vault_tokens) - i)) for i in range(len(vault_tokens))],
            [0 for _ in vault_tokens],
            {"from": berg},
        )
    except VirtualMachineError:
        assert unbalance_percentage + withdrawal_percentage > 1
        # TODO: Handle
        return

    withdrawMixedAmount = tx_mixed.return_value

    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        if mixedAmount > allAmount:
            assert mixedAmount <= int(
                allAmount * (1 + 1e-10) + 1
            )  # If more is returned, it needs to be almost insignificant. (and there is a deposit fee.)
        assert int(allAmount * 99 / 100) <= mixedAmount
