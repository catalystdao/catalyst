import pytest
from brownie import reverts
from brownie.test import given
from hypothesis.strategies import floats

from tests.catalyst.utils.common_utils import assert_abs_relative_error


@given(swap_amount_percentage=floats(min_value=0, max_value=1))    # From 0 to 1x the tokens hold by the pool
def test_local_swap(
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
    
    y = compute_expected_local_swap(swap_amount, source_token, target_token)["output"]
    
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
    
    
@given(swap_amount_percentage=floats(min_value=0.1, max_value=1))    # From 0.1x to 1x the tokens hold by the pool
def test_local_swap_minout_always_fails(
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

    swap_amount = swap_amount_percentage * source_token.balanceOf(pool)

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(pool, swap_amount, {'from': berg})
    
    y = compute_expected_local_swap(swap_amount, source_token, target_token)["output"]
    
    with reverts("Insufficient Return"):
        pool.localswap(
            source_token, target_token, swap_amount, y*1.1, {'from': berg}
        )


@given(
    swap_amount_percentage=floats(min_value=0, max_value=1),
    min_out_percentage=floats(min_value=0, max_value=1)
)
def test_local_swap_minout(
    pool,
    pool_tokens,
    berg,
    deployer,
    swap_amount_percentage,
    min_out_percentage
):
    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")

    source_token = pool_tokens[0]
    target_token = pool_tokens[1]

    swap_amount = swap_amount_percentage * source_token.balanceOf(pool)
    min_out     = min_out_percentage * target_token.balanceOf(pool)

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(pool, swap_amount, {'from': berg})
    
    simulated_swap_return = pool.dry_swap_both(source_token, target_token, swap_amount)
    
    if simulated_swap_return < min_out:
        with reverts("Insufficient Return"):
            pool.localswap(
                source_token, target_token, swap_amount, min_out, {'from': berg}
            )
    else:
        tx = pool.localswap(
            source_token, target_token, swap_amount, min_out, {'from': berg}
        )
        assert min_out <= tx.return_value


def test_local_swap_event(pool, pool_tokens, berg, deployer):
    """
        Test the LocalSwap event gets fired.
    """

    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")

    swap_amount = 10**8

    source_token = pool_tokens[0]
    target_token = pool_tokens[1]

    source_token.transfer(berg, swap_amount, {'from': deployer})      # Fund berg's account with tokens to swap
    source_token.approve(pool, swap_amount, {'from': berg})
    
    tx = pool.localswap(source_token, target_token, swap_amount, 0, {'from': berg})

    observed_return = tx.return_value

    swap_event = tx.events['LocalSwap']

    assert swap_event['who']       == berg
    assert swap_event['fromAsset'] == source_token
    assert swap_event['toAsset']   == target_token
    assert swap_event['input']     == swap_amount
    assert swap_event['output']    == observed_return



@given(swap_amount_percentage=floats(min_value=0, max_value=1))    # From 0 to 1x the tokens hold by the pool
@pytest.mark.usefixtures("pool_set_fees")
def test_local_swap_fees(
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
    init_gov_source_balance  = source_token.balanceOf(deployer)     #TODO replace the 'deployer' account with a 'governance' fixture (or rename deployer to governance)

    swap_amount = swap_amount_percentage * init_pool_source_balance

    assert target_token.balanceOf(berg) == 0
    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(pool, swap_amount, {'from': berg})
    
    expected_swap_result = compute_expected_local_swap(swap_amount, source_token, target_token)
    

    tx = pool.localswap(
        source_token, target_token, swap_amount, 0, {'from': berg}
    )


    assert tx.return_value <= int(expected_swap_result["output"] * 1.000001), "Swap returns more than theoretical"
    assert tx.return_value >= int(expected_swap_result["output"] * 9/10), "Swap returns less than 9/10 theoretical"
    
    # Verify user token balances
    assert source_token.balanceOf(berg) == 0
    assert target_token.balanceOf(berg) == tx.return_value

    # Verify pool balances
    pool_fee = expected_swap_result["pool_fee"] # TODO how do we verify the correctness of this? Assert change (increase) in pool invariant? Or is the expected swap return enough?
    governance_fee = expected_swap_result["governance_fee"]

    assert_abs_relative_error(
        source_token.balanceOf(pool),
        init_pool_source_balance + swap_amount - governance_fee,  # Governance fee is sent directly to the governance account
        1e-15
    )
    assert target_token.balanceOf(pool) == init_pool_target_balance - tx.return_value

    # Verify governance balances
    assert_abs_relative_error(
        source_token.balanceOf(deployer),
        init_gov_source_balance + governance_fee,
        1e-15
    )
