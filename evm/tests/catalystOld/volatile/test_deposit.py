import pytest
from brownie import reverts
from brownie.test import given, strategy


# This function tests the depositMixed function against an older implementation called deposit all.
@given(percentage=strategy("uint256", max_value=10000))
def test_deposit_like_all(swappool, get_pool_tokens, berg, deployer, percentage):
    percentage /= 10000
    
    tokens = get_pool_tokens(swappool)
    amounts = [int(token.balanceOf(swappool) * percentage) for token in tokens]
    [token.transfer(berg, amount, {'from': deployer}) for token, amount in zip(tokens, amounts)]
    [token.approve(swappool, amount, {'from': berg}) for token, amount in zip(tokens, amounts)]
    
    estimatedPoolTokens = int(swappool.totalSupply()*percentage)
    
    tx = swappool.depositMixed(amounts, 0, {'from': berg})
    
    assert int(estimatedPoolTokens * 999 / 1000) <= tx.return_value, "Deposit returns less 999/1000 of theoretical"
    assert tx.return_value <= estimatedPoolTokens, "Deposit returns more than theoretical"
    
    
    
    