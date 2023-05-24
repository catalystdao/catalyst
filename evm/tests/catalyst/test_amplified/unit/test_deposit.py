import pytest
from brownie.test import given, strategy
from hypothesis import example


@example(percentage=2 * 10**5)
@given(percentage=strategy("uint256", max_value=1 * 10**6))
def test_deposit_equi_like_all(vault, vault_tokens, berg, deployer, percentage):
    percentage /= 10**6
    # Deposit when the vault is balanced. That is: The balances are equal to balance0.

    amounts = [int(token.balanceOf(vault) * percentage) for token in vault_tokens]
    [
        token.transfer(berg, amount, {"from": deployer})
        for token, amount in zip(vault_tokens, amounts)
    ]
    [
        token.approve(vault, amount, {"from": berg})
        for token, amount in zip(vault_tokens, amounts)
    ]

    estimatedVaultTokens = int(vault.totalSupply() * percentage)

    tx = vault.depositMixed(amounts, 0, {"from": berg})

    assert (
        int(estimatedVaultTokens * 999 / 1000) <= tx.return_value
    ), "Deposit returns less 999/1000 of theoretical"
    # 0,0001% error is allowed on an upside. Any sane vault should implement a fee greater than this.
    # in which case the fee eats any potential upside.
    assert tx.return_value <= int(
        estimatedVaultTokens * 1.00001
    ), "Deposit returns more than theoretical"
