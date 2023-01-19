import pytest
from brownie.test import given, strategy


@given(percentage=strategy("uint256", max_value=1*10**6))
def test_deposit_equi_like_all(pool, pool_tokens, berg, deployer, percentage):
    percentage /= 10**6
    # Deposit when the pool is balanced. That is: The balances are equal to balance0.
    
    amounts = [int(token.balanceOf(pool) * percentage) for token in pool_tokens]
    [token.transfer(berg, amount, {'from': deployer}) for token, amount in zip(pool_tokens, amounts)]
    [token.approve(pool, amount, {'from': berg}) for token, amount in zip(pool_tokens, amounts)]
    
    estimatedPoolTokens = int(pool.totalSupply()*percentage)
    
    tx = pool.depositMixed(amounts, 0, {'from': berg})
    
    assert int(estimatedPoolTokens * 999 / 1000) <= tx.return_value, "Deposit returns less 999/1000 of theoretical"
    # 0,0001% error is allowed on an upside. Any sane pool should implement a fee greater than this.
    # in which case the fee eats any potential upside.
    assert tx.return_value <= int(estimatedPoolTokens * 1.00001), "Deposit returns more than theoretical"
    
