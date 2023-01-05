import pytest
from brownie import chain
from brownie.test import given, strategy


# This function compares the output difference between withdrawAll and withdrawMixed
@given(percentage=strategy("uint256", min_value=100, max_value=10000))
def test_withdrawlike_all(swappool, get_pool_tokens, berg, molly, percentage):
    percentage /= 10000
    
    # Check if the test is valid
    tokens = get_pool_tokens(swappool)
    weights = [swappool._weight(token) for token in tokens]
    if sum([sum(weights)/len(weights) - weight for weight in weights]) != 0:
        return  # Unequal weights are not implemented
    
    # Lets continue
    poolTokens = int(swappool.balanceOf(molly) * percentage)
    swappool.transfer(berg, poolTokens, {'from': molly})
    
    tx_all = swappool.withdrawAll(poolTokens, [0 for _ in tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    chain.undo()
    
    tx_mixed = swappool.withdrawMixed(poolTokens, [int(2**64/(len(tokens) - i)) for i in range(len(tokens))], [0 for _ in tokens], {'from': berg})
    
    withdrawMixedAmount = tx_mixed.return_value
    
    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        assert mixedAmount <= allAmount
        assert int(allAmount * 7 / 10) <= mixedAmount
    
    
    
    
    
    