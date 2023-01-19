import pytest
from brownie import chain
from brownie.test import given, strategy


# This function compares the output difference between withdrawAll and withdrawMixed
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
@pytest.mark.no_call_coverage
def test_withdrawall(pool, pool_tokens, berg, deployer, percentage):
    percentage /= 10000
    
    poolBalances = [token.balanceOf(pool) for token in pool_tokens]
    poolTokens = int(pool.balanceOf(deployer) * percentage)
    pool.transfer(berg, poolTokens, {'from': deployer})
    ts = pool.totalSupply()
    
    tx_all = pool.withdrawAll(poolTokens, [0 for _ in pool_tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    
    for allAmount, poolBalance in zip(withdrawAllAmount, poolBalances):
        assert allAmount <= poolBalance*poolTokens // ts
        assert int(poolBalance * percentage * 9 / 10) <= allAmount
    

# This function compares the output difference between withdrawAll and withdrawMixed
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
@pytest.mark.no_call_coverage
def test_compare_withdrawall_and_withdrawmixed(pool, pool_tokens, berg, deployer, percentage):
    percentage /= 10000
    
    poolTokens = int(pool.balanceOf(deployer) * percentage)
    pool.transfer(berg, poolTokens, {'from': deployer})
    
    tx_all = pool.withdrawAll(poolTokens, [0 for _ in pool_tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    chain.undo()
    
    tx_mixed = pool.withdrawMixed(poolTokens, [int(10**18/(len(pool_tokens) - i)) for i in range(len(pool_tokens))], [0 for _ in pool_tokens], {'from': berg})
    
    withdrawMixedAmount = tx_mixed.return_value
    
    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        assert mixedAmount <= allAmount
        assert int(allAmount * 7 / 10) <= mixedAmount
    
    
    
    
    
    