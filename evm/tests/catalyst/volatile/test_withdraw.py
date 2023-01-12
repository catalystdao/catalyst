import pytest
from brownie import chain
from brownie.test import given, strategy


# This function compares the output difference between withdrawAll and withdrawMixed
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
def test_withdrawall(swappool, get_pool_tokens, berg, molly, percentage):
    percentage /= 10000
    
    tokens = get_pool_tokens(swappool)
    poolBalances = [token.balanceOf(swappool) for token in tokens]
    poolTokens = int(swappool.balanceOf(molly) * percentage)
    swappool.transfer(berg, poolTokens, {'from': molly})
    ts = swappool.totalSupply()
    
    tx_all = swappool.withdrawAll(poolTokens, [0 for _ in tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    
    for allAmount, poolBalance in zip(withdrawAllAmount, poolBalances):
        assert allAmount <= poolBalance*poolTokens // ts
        assert int(poolBalance * percentage * 9 / 10) <= allAmount
    

# This function compares the output difference between withdrawAll and withdrawMixed
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
def test_compare_withdrawall_and_withdrawmixed(swappool, get_pool_tokens, berg, molly, percentage):
    percentage /= 10000
    
    tokens = get_pool_tokens(swappool)
    poolTokens = int(swappool.balanceOf(molly) * percentage)
    swappool.transfer(berg, poolTokens, {'from': molly})
    
    tx_all = swappool.withdrawAll(poolTokens, [0 for _ in tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    chain.undo()
    
    tx_mixed = swappool.withdrawMixed(poolTokens, [int(10**18/(len(tokens) - i)) for i in range(len(tokens))], [0 for _ in tokens], {'from': berg})
    
    withdrawMixedAmount = tx_mixed.return_value
    
    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        assert mixedAmount <= allAmount
        assert int(allAmount * 7 / 10) <= mixedAmount
    
    
    
    
    
    