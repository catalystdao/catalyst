import pytest
from brownie import reverts

@pytest.fixture(scope="module")
def berg_pool_token_balance(pool, pool_tokens, deployer, berg):

    deposit_percentage = 0.15
    deposit_amounts = [int(token.balanceOf(pool) * deposit_percentage) for token in pool_tokens]
    for token, amount in zip(pool_tokens, deposit_amounts):
        token.transfer(berg, amount, {"from": deployer})
        token.approve(pool, amount, {"from": berg})
    
    tx = pool.depositMixed(deposit_amounts, 0, {"from": berg})

    return tx.return_value



def test_pool_token_total_supply_query(
    pool,
    berg_pool_token_balance
):
    assert pool.totalSupply() == berg_pool_token_balance + 10**18       # NOTE: 10**18 is the pool token supply given to the pool deployer



def test_pool_token_balance_query(
    pool,
    berg,
    berg_pool_token_balance
):
    assert pool.balanceOf(berg) == berg_pool_token_balance



def test_pool_token_transfer(
    pool,
    berg,
    molly,
    berg_pool_token_balance
):
    assert pool.balanceOf(berg) == berg_pool_token_balance
    assert pool.balanceOf(molly) == 0

    transfer_amount = int(0.2 * berg_pool_token_balance)


    tx = pool.transfer(molly, transfer_amount, {"from": berg})


    assert pool.balanceOf(berg) == berg_pool_token_balance - transfer_amount
    assert pool.balanceOf(molly) == transfer_amount

    event = tx.events["Transfer"]
    assert event["from"] == berg
    assert event["to"] == molly
    assert event["value"] == transfer_amount



def test_pool_token_transfer_no_balance(
    pool,
    berg,
    molly,
    berg_pool_token_balance
):
    assert pool.balanceOf(berg) == berg_pool_token_balance
    assert pool.balanceOf(molly) == 0

    transfer_amount = int(1.1*berg_pool_token_balance)


    with reverts():
        pool.transfer(molly, transfer_amount, {"from": berg})



def test_pool_token_set_and_query_allowance(
    pool,
    berg,
    molly,
    berg_pool_token_balance
):
    assert pool.allowance(berg, molly) == 0

    allowance_amount = int(0.2 * berg_pool_token_balance)


    tx = pool.approve(molly, allowance_amount, {"from": berg})


    assert pool.allowance(berg, molly) == allowance_amount

    event = tx.events["Approval"]
    assert event["owner"] == berg
    assert event["spender"] == molly
    assert event["value"] == allowance_amount



def test_pool_token_remove_allowance(
    pool,
    berg,
    molly,
    berg_pool_token_balance
):

    init_allowance_amount = int(0.2 * berg_pool_token_balance)
    pool.approve(molly, init_allowance_amount, {"from": berg})

    assert pool.allowance(berg, molly) == init_allowance_amount


    tx = pool.approve(molly, 0, {"from": berg})


    assert pool.allowance(berg, molly) == 0

    event = tx.events["Approval"]
    assert event["owner"] == berg
    assert event["spender"] == molly
    assert event["value"] == 0



def test_pool_token_increase_allowance(
    pool,
    berg,
    molly,
    berg_pool_token_balance
):

    init_allowance_amount = int(0.2 * berg_pool_token_balance)
    pool.approve(molly, init_allowance_amount, {"from": berg})

    assert pool.allowance(berg, molly) == init_allowance_amount

    increase_allowance_amount = int(0.25 * init_allowance_amount)


    tx = pool.increaseAllowance(molly, increase_allowance_amount, {"from": berg})


    assert pool.allowance(berg, molly) == init_allowance_amount + increase_allowance_amount

    event = tx.events["Approval"]
    assert event["owner"] == berg
    assert event["spender"] == molly
    assert event["value"] == init_allowance_amount + increase_allowance_amount



def test_pool_token_decrease_allowance(
    pool,
    berg,
    molly,
    berg_pool_token_balance
):

    init_allowance_amount = int(0.2 * berg_pool_token_balance)
    pool.approve(molly, init_allowance_amount, {"from": berg})

    assert pool.allowance(berg, molly) == init_allowance_amount

    decrease_allowance_amount = int(0.25 * init_allowance_amount)


    tx = pool.decreaseAllowance(molly, decrease_allowance_amount, {"from": berg})


    assert pool.allowance(berg, molly) == init_allowance_amount - decrease_allowance_amount

    event = tx.events["Approval"]
    assert event["owner"] == berg
    assert event["spender"] == molly
    assert event["value"] == init_allowance_amount - decrease_allowance_amount



def test_pool_token_transfer_from(
    pool,
    berg,
    molly,
    elwood,
    berg_pool_token_balance
):

    allowance_amount = int(0.2 * berg_pool_token_balance)
    pool.approve(molly, allowance_amount, {"from": berg})

    assert pool.balanceOf(elwood) == 0

    transfer_amount = int(0.33 * allowance_amount)


    tx = pool.transferFrom(berg, elwood, transfer_amount, {"from": molly})


    assert pool.balanceOf(berg) == berg_pool_token_balance - transfer_amount
    assert pool.balanceOf(elwood) == transfer_amount
    assert pool.allowance(berg, molly) == allowance_amount - transfer_amount

    event = tx.events["Transfer"]
    assert event["from"] == berg
    assert event["to"] == elwood
    assert event["value"] == transfer_amount



def test_pool_token_transfer_from_no_allowance(
    pool,
    berg,
    molly,
    elwood,
    berg_pool_token_balance
):

    allowance_amount = int(0.2 * berg_pool_token_balance)
    pool.approve(molly, allowance_amount, {"from": berg})

    assert pool.balanceOf(elwood) == 0

    transfer_amount = int(1.1 * allowance_amount)


    with reverts():
        pool.transferFrom(berg, elwood, transfer_amount, {"from": molly})
