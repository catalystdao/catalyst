import pytest
from brownie import reverts, convert, ZERO_ADDRESS, chain
from brownie.test import given, strategy
from hypothesis.strategies import floats
import re
    
    
def test_get_compare_mid_price_with_small_swap(
    catalyst_describer_filled, 
    pool,
    pool_tokens,
    berg,
    deployer,
):
    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")
    if len(pool_tokens) > 3:
        pytest.skip("Need at less than 3 tokens within a pool to run a local swap.")
        
    mathlib = catalyst_describer_filled.get_vault_mathematical_lib(pool)
    
    prices = catalyst_describer_filled.get_vault_prices(pool)
    
    swaps = []
    for i in range(len(prices) - 1):
        # i == 0: 0 -> 1
        # i == 1: 1 -> 2
        
        swap_amount = pool_tokens[0].balanceOf(pool) // 1000
        
        pool_tokens[i].transfer(berg, swap_amount, {'from': deployer})
        pool_tokens[i].approve(pool, swap_amount, {'from': berg})
        
        swaps.append(
            pool.localSwap(
                pool_tokens[i], pool_tokens[i + 1], swap_amount, 0, {'from': berg}
            ).return_value/swap_amount
        )
        chain.undo()
    
    for i in range(len(prices) - 1):
        computedPrice = prices[i+1]/prices[i]
        observed_price = swaps[i]

        assert 1 <= computedPrice / observed_price <= 1.0011
        
