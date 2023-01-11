import pytest
from brownie import reverts
from brownie.test import given, strategy

#TODO add fees test (create fixture that sets up non-zero fees to the pool)


@given(swap_amount=strategy("uint256", max_value=2000*10**18))
def test_local_swap(swappool, token1, token2, berg, deployer, compute_expected_swap, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token2, swappool)
    
    tx = swappool.localswap(
         token1, token2, swap_amount, 0, {'from': berg}
    )
    assert token1.balanceOf(berg) == 0
    
    assert tx.return_value <= int(y*1.000001), "Swap returns more than theoretical"
    assert (y * 9 /10) <= tx.return_value, "Swap returns less than 9/10 theoretical"
    
    assert tx.return_value == token2.balanceOf(berg)
    
    

@given(swap_amount=strategy("uint256", max_value=2000*10**18))
def test_local_swap_approx(swappool, token1, token2, berg, deployer, compute_expected_swap, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token2, swappool)
    
    if swap_amount/(token1.balanceOf(swappool)-swap_amount) < 1e-02:
        tx = swappool.localswap(
            token1, token2, swap_amount, 0, True, {'from': berg}
        )
        assert tx.return_value <= int(y*1.000001), "Swap returns more than theoretical"
    else:
        tx = swappool.localswap(
            token1, token2, swap_amount, (y * 9 /10), True, {'from': berg}
        )
        assert tx.return_value <= int(y*1.000001), "Swap returns more than theoretical"
        assert (y * 9 /10) <= tx.return_value, "Swap returns less than 9/10 theoretical"
    
    
@given(swap_amount=strategy("uint256", min_value=1*10**18, max_value=2000*10**18))
def test_local_swap_minout_always_fails(swappool, token1, token2, berg, deployer, compute_expected_swap, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token2, swappool)
    
    with reverts("Insufficient Return"):
        tx = swappool.localswap(
            token1, token2, swap_amount, y*1.1, {'from': berg}
        )


@given(swap_amount=strategy("uint256", max_value=2000*10**18), min_out=strategy("uint256", max_value=2000*10**18))
def test_local_swap_minout(swappool, token1, token2, berg, deployer, swap_amount, min_out):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool, swap_amount, {'from': berg})
    
    simulated_swap_return = swappool.dry_swap_both(token1, token2, swap_amount, False)
    
    if simulated_swap_return < min_out:
        with reverts("Insufficient Return"):
            tx = swappool.localswap(
                token1, token2, swap_amount, min_out, False, {'from': berg}
            )
    else:
        tx = swappool.localswap(
            token1, token2, swap_amount, min_out, False, {'from': berg}
        )
        assert min_out <= tx.return_value


def test_local_swap_event(swappool, token1, token2, berg, deployer):
    """
        Test the LocalSwap event gets fired.
    """

    swap_amount = 10**8

    token1.transfer(berg, swap_amount, {'from': deployer})      # Fund berg's account with tokens to swap
    token1.approve(swappool, swap_amount, {'from': berg})
    
    tx = swappool.localswap(token1, token2, swap_amount, 0, {'from': berg})

    observed_return = tx.return_value

    swap_event = tx.events['LocalSwap']

    assert swap_event['who']       == berg
    assert swap_event['fromAsset'] == token1
    assert swap_event['toAsset']   == token2
    assert swap_event['input']     == swap_amount
    assert swap_event['output']    == observed_return
    assert swap_event['fees']      >= 0                     # Check that there is a fees field


def test_local_swap_event_approx(swappool, token1, token2, berg, deployer):
    """
        Test the LocalSwap event gets fired (approx swap).
    """

    swap_amount = 10**8

    token1.transfer(berg, swap_amount, {'from': deployer})      # Fund berg's account with tokens to swap
    token1.approve(swappool, swap_amount, {'from': berg})
    
    tx = swappool.localswap(token1, token2, swap_amount, 0, True, {'from': berg})

    observed_return = tx.return_value

    swap_event = tx.events['LocalSwap']

    assert swap_event['who']       == berg
    assert swap_event['fromAsset'] == token1
    assert swap_event['toAsset']   == token2
    assert swap_event['input']     == swap_amount
    assert swap_event['output']    == observed_return
    assert swap_event['fees']      >= 0                     # Check that there is a fees field