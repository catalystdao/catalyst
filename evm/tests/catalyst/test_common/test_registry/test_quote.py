import pytest
from brownie import reverts, convert, ZERO_ADDRESS, chain
from brownie.test import given, strategy
from hypothesis.strategies import floats
import re
    
    
def test_get_compare_mid_price_with_small_swap(
    catalyst_describer_filled, 
    vault,
    vault_tokens,
    berg,
    deployer,
):
    if len(vault_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a vault to run a local swap.")
    if len(vault_tokens) > 3:
        pytest.skip("Need at less than 3 tokens within a vault to run a local swap.")
        
    mathlib = catalyst_describer_filled.get_vault_mathematical_lib(vault)
    
    prices = catalyst_describer_filled.get_vault_prices(vault)
    
    swaps = []
    for i in range(len(prices) - 1):
        # i == 0: 0 -> 1
        # i == 1: 1 -> 2
        
        swap_amount = vault_tokens[0].balanceOf(vault) // 1000
        
        vault_tokens[i].transfer(berg, swap_amount, {'from': deployer})
        vault_tokens[i].approve(vault, swap_amount, {'from': berg})
        
        swaps.append(
            vault.localSwap(
                vault_tokens[i], vault_tokens[i + 1], swap_amount, 0, {'from': berg}
            ).return_value/swap_amount
        )
        chain.undo()
    
    for i in range(len(prices) - 1):
        computedPrice = prices[i+1]/prices[i]
        observed_price = swaps[i]

        assert 1 <= computedPrice / observed_price <= 1.0011
        
