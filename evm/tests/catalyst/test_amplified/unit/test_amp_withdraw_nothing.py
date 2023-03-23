import pytest
from brownie import chain, reverts
from brownie.test import given, strategy
from hypothesis import example
from brownie.exceptions import VirtualMachineError
import tests.catalyst.utils.pool_utils as pool_utils

@pytest.mark.no_call_coverage
def test_withdraw_nothing(pool, pool_tokens, berg, deployer):

    for token in pool_tokens:
        assert token.balanceOf(berg) <= 0
    
    tx_all = pool.withdrawAll(0, [0 for _ in pool_tokens], {'from': berg})
    
    for token in pool_tokens:
        assert token.balanceOf(berg) <= 0
        
    chain.undo()
    
    tx_mixed = pool.withdrawMixed(0, [0 for _ in pool_tokens], {'from': berg})
    
    for token in pool_tokens:
        assert token.balanceOf(berg) <= 0
    
    
@pytest.mark.no_call_coverage
def test_withdraw_almost_nothing(pool, pool_tokens, berg, deployer):

    ts = pool.totalSupply()
    
    pool.transfer(berg, 2, {'from': deployer})
    
    for token in pool_tokens:
        assert token.balanceOf(berg) <= token.balanceOf(pool) * 2 // ts
    
    tx_all = pool.withdrawAll(2, [0 for _ in pool_tokens], {'from': berg})
    
    for token in pool_tokens:
        assert token.balanceOf(berg) <= token.balanceOf(pool) * 2 // ts
        
    chain.undo()
    
    tx_mixed = pool.withdrawMixed(2, [0 for _ in pool_tokens], {'from': berg})
    
    for token in pool_tokens:
        assert token.balanceOf(berg) <= token.balanceOf(pool) * 2 // ts
    
    