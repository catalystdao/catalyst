import pytest
from brownie import chain, reverts
from brownie.test import given, strategy
from hypothesis import example
from brownie.exceptions import VirtualMachineError
import tests.catalyst.utils.vault_utils as vault_utils


@pytest.mark.no_call_coverage
def test_withdraw_nothing(vault, vault_tokens, berg, deployer):

    for token in vault_tokens:
        assert token.balanceOf(berg) <= 0

    tx_all = vault.withdrawAll(0, [0 for _ in vault_tokens], {"from": berg})

    for token in vault_tokens:
        assert token.balanceOf(berg) <= 0

    chain.undo()

    tx_mixed = vault.withdrawMixed(
        0, [0, 0, 0], [0 for _ in vault_tokens], {"from": berg}
    )

    for token in vault_tokens:
        assert token.balanceOf(berg) <= 0


@pytest.mark.no_call_coverage
def test_withdraw_almost_one(vault, vault_tokens, berg, deployer):
    token_withdraw_ratios = [
        int(10**18 / (len(vault_tokens) - i)) for i in range(len(vault_tokens))
    ]

    ts = vault.totalSupply()

    vault.transfer(berg, 1, {"from": deployer})

    for token in vault_tokens:
        assert (
            token.balanceOf(berg)
            <= (token.balanceOf(vault) + token.balanceOf(berg)) * 1 // ts
        )

    tx_all = vault.withdrawAll(1, [0 for _ in vault_tokens], {"from": berg})

    for token in vault_tokens:
        assert (
            token.balanceOf(berg)
            <= (token.balanceOf(vault) + token.balanceOf(berg)) * 1 // ts
        )

    chain.undo()

    with reverts():
        tx_mixed = vault.withdrawMixed(
            1, token_withdraw_ratios, [0 for _ in vault_tokens], {"from": berg}
        )

    for token in vault_tokens:
        assert (
            token.balanceOf(berg)
            <= (token.balanceOf(vault) + token.balanceOf(berg)) * 1 // ts
        )


@pytest.mark.no_call_coverage
def test_withdraw_almost_two(vault, vault_tokens, berg, deployer):
    token_withdraw_ratios = [
        int(10**18 / (len(vault_tokens) - i)) for i in range(len(vault_tokens))
    ]

    ts = vault.totalSupply()

    vault.transfer(berg, 2, {"from": deployer})

    for token in vault_tokens:
        assert (
            token.balanceOf(berg)
            <= (token.balanceOf(vault) + token.balanceOf(berg)) * 2 // ts
        )

    tx_all = vault.withdrawAll(2, [0 for _ in vault_tokens], {"from": berg})

    for token in vault_tokens:
        assert (
            token.balanceOf(berg)
            <= (token.balanceOf(vault) + token.balanceOf(berg)) * 2 // ts
        )

    chain.undo()

    tx_mixed = vault.withdrawMixed(
        2, token_withdraw_ratios, [0 for _ in vault_tokens], {"from": berg}
    )

    for token in vault_tokens:
        assert (
            token.balanceOf(berg)
            <= (token.balanceOf(vault) + token.balanceOf(berg)) * 2 // ts
        )
