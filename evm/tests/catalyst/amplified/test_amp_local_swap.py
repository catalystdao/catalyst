import pytest
from brownie import reverts, chain
from brownie.test import given, strategy


@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


@given(swap_amount=strategy("uint256", max_value=10*10**18))
def test_local_swap(amp_swappool, token1, token2, berg, deployer, compute_expected_swap, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    
    y = compute_expected_swap(swap_amount, token1, token2, amp_swappool)
    
    token1.approve(amp_swappool, swap_amount, {'from': deployer})
    
    tx = amp_swappool.localswap(
         token1, token2, swap_amount, 0, {'from': deployer}
    )
    
    if swap_amount/(token1.balanceOf(amp_swappool)-swap_amount) < 1e-02:
        assert tx.return_value <= y
    else:
        assert tx.return_value <= int(y*1.000001), "Swap returns more than theoretical"
        assert (y * 9 /10) <= tx.return_value, "Swap returns less than 9/10 theoretical"
    
    
@given(swap_amount=strategy("uint256", min_value=1*10**18, max_value=10*10**18))
def test_local_swap_minout_always_fails(amp_swappool, token1, token2, berg, deployer, compute_expected_swap, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    
    y = compute_expected_swap(swap_amount, token1, token2, amp_swappool)
    
    token1.approve(amp_swappool, swap_amount, {'from': deployer})
    
    with reverts("Insufficient Return"):
        tx = amp_swappool.localswap(
            token1, token2, swap_amount, y*1.1, {'from': deployer}
        )


@given(swap_amount=strategy("uint256", max_value=10*10**18), min_out=strategy("uint256", max_value=1000*10**18))
def test_local_swap_minout(amp_swappool, token1, token2, berg, deployer, swap_amount, min_out):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(amp_swappool, swap_amount, {'from': deployer})
    
    simulated_swap_return = amp_swappool.dry_swap_both(token1, token2, swap_amount)
    
    if simulated_swap_return < min_out:
        with reverts("Insufficient Return"):
            tx = amp_swappool.localswap(
                token1, token2, swap_amount, min_out, {'from': deployer}
            )
    else:
        tx = amp_swappool.localswap(
            token1, token2, swap_amount, min_out, {'from': deployer}
        )
        assert min_out <= tx.return_value
    