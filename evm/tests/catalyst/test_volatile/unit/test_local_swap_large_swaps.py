import pytest
from brownie.test import given
from hypothesis.strategies import floats
from hypothesis import example


# ! Only testing the accuracy of large swaps. For the rest of the local swap tests, see test_local_swap.py under test_common/
@example(swap_amount_percentage=4.7)
@given(swap_amount_percentage=floats(min_value=1, max_value=10))    # From 1x to 10x the tokens hold by the pool
def test_local_swap_large_swaps(
    pool,
    pool_tokens,
    berg,
    deployer,
    compute_expected_local_swap,
    swap_amount_percentage
):
    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")

    source_token = pool_tokens[0]
    target_token = pool_tokens[1]

    init_pool_source_balance = source_token.balanceOf(pool)
    init_pool_target_balance = target_token.balanceOf(pool)

    swap_amount = swap_amount_percentage * init_pool_source_balance

    assert target_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(pool, swap_amount, {'from': berg})
    
    y = compute_expected_local_swap(swap_amount, source_token, target_token)["to_amount"]
    
    tx = pool.localswap(
        source_token, target_token, swap_amount, 0, {'from': berg}
    )
    
    assert tx.return_value <= int(y*1.000001), "Swap returns more than theoretical"
    assert (y * 9 /10) <= tx.return_value, "Swap returns less than 9/10 theoretical"
    
    # Verify user token balances
    assert source_token.balanceOf(berg) == 0
    assert target_token.balanceOf(berg) == tx.return_value

    # Verify pool token balances
    assert source_token.balanceOf(pool) == init_pool_source_balance + swap_amount
    assert target_token.balanceOf(pool) == init_pool_target_balance - tx.return_value
    