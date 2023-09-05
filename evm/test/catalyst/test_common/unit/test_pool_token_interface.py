import pytest
from brownie import reverts

pytestmark = pytest.mark.no_vault_param


@pytest.fixture(scope="module")
def berg_vault_token_balance(vault, vault_tokens, deployer, berg):

    deposit_percentage = 0.15
    deposit_amounts = [
        int(token.balanceOf(vault) * deposit_percentage) for token in vault_tokens
    ]
    for token, amount in zip(vault_tokens, deposit_amounts):
        token.transfer(berg, amount, {"from": deployer})
        token.approve(vault, amount, {"from": berg})

    tx = vault.depositMixed(deposit_amounts, 0, {"from": berg})

    return tx.return_value


def test_vault_token_total_supply_query(vault, berg_vault_token_balance):
    assert (
        vault.totalSupply() == berg_vault_token_balance + 10**18
    )  # NOTE: 10**18 is the vault token supply given to the vault deployer


def test_vault_token_balance_query(vault, berg, berg_vault_token_balance):
    assert vault.balanceOf(berg) == berg_vault_token_balance


def test_vault_token_transfer(vault, berg, molly, berg_vault_token_balance):
    assert vault.balanceOf(berg) == berg_vault_token_balance
    assert vault.balanceOf(molly) == 0

    transfer_amount = int(0.2 * berg_vault_token_balance)

    tx = vault.transfer(molly, transfer_amount, {"from": berg})

    assert vault.balanceOf(berg) == berg_vault_token_balance - transfer_amount
    assert vault.balanceOf(molly) == transfer_amount

    event = tx.events["Transfer"]
    assert event["from"] == berg
    assert event["to"] == molly
    assert event["amount"] == transfer_amount


def test_vault_token_transfer_no_balance(vault, berg, molly, berg_vault_token_balance):
    assert vault.balanceOf(berg) == berg_vault_token_balance
    assert vault.balanceOf(molly) == 0

    transfer_amount = int(1.1 * berg_vault_token_balance)

    with reverts():
        vault.transfer(molly, transfer_amount, {"from": berg})


def test_vault_token_set_and_query_allowance(
    vault, berg, molly, berg_vault_token_balance
):
    assert vault.allowance(berg, molly) == 0

    allowance_amount = int(0.2 * berg_vault_token_balance)

    tx = vault.approve(molly, allowance_amount, {"from": berg})

    assert vault.allowance(berg, molly) == allowance_amount

    event = tx.events["Approval"]
    assert event["owner"] == berg
    assert event["spender"] == molly
    assert event["amount"] == allowance_amount


def test_vault_token_remove_allowance(vault, berg, molly, berg_vault_token_balance):

    init_allowance_amount = int(0.2 * berg_vault_token_balance)
    vault.approve(molly, init_allowance_amount, {"from": berg})

    assert vault.allowance(berg, molly) == init_allowance_amount

    tx = vault.approve(molly, 0, {"from": berg})

    assert vault.allowance(berg, molly) == 0

    event = tx.events["Approval"]
    assert event["owner"] == berg
    assert event["spender"] == molly
    assert event["amount"] == 0


def test_vault_token_transfer_from(
    vault, berg, molly, elwood, berg_vault_token_balance
):

    allowance_amount = int(0.2 * berg_vault_token_balance)
    vault.approve(molly, allowance_amount, {"from": berg})

    assert vault.balanceOf(elwood) == 0

    transfer_amount = int(0.33 * allowance_amount)

    tx = vault.transferFrom(berg, elwood, transfer_amount, {"from": molly})

    assert vault.balanceOf(berg) == berg_vault_token_balance - transfer_amount
    assert vault.balanceOf(elwood) == transfer_amount
    assert vault.allowance(berg, molly) == allowance_amount - transfer_amount

    event = tx.events["Transfer"]
    assert event["from"] == berg
    assert event["to"] == elwood
    assert event["amount"] == transfer_amount


def test_vault_token_transfer_from_no_allowance(
    vault, berg, molly, elwood, berg_vault_token_balance
):

    allowance_amount = int(0.2 * berg_vault_token_balance)
    vault.approve(molly, allowance_amount, {"from": berg})

    assert vault.balanceOf(elwood) == 0

    transfer_amount = int(1.1 * allowance_amount)

    with reverts():
        vault.transferFrom(berg, elwood, transfer_amount, {"from": molly})
