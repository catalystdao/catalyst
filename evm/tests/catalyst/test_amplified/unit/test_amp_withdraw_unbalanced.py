import pytest
from brownie import chain, reverts
from brownie.test import given, strategy
from hypothesis import example
from brownie.exceptions import VirtualMachineError
import tests.catalyst.utils.pool_utils as pool_utils


@example(unbalance_percentage=8000, withdrawal_percentage=9000)
@given(unbalance_percentage=strategy("uint256", min_value=100, max_value=10000), withdrawal_percentage=strategy("uint256", min_value=100, max_value=10000))
@pytest.mark.no_call_coverage
def test_withdrawall(pool, pool_tokens, get_pool_amp, berg, deployer, unbalance_percentage, withdrawal_percentage):
    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")
    
    unbalance_percentage /= 10000
    withdrawal_percentage /= 10000
    amp = get_pool_amp()
    
    poolBalances = [token.balanceOf(pool) for token in pool_tokens]
    weights = [pool._weight(token) for token in pool_tokens]
    swap_amount = int(poolBalances[0] * unbalance_percentage)
    poolTokens = int(pool.totalSupply() * withdrawal_percentage)
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})
    pool.transfer(berg, poolTokens, {'from': deployer})
    
    tx_swap = pool.localSwap(
        pool_tokens[0], pool_tokens[1], swap_amount, 0, {'from': berg}
    )
    
    poolBalances = [token.balanceOf(pool) for token in pool_tokens]
    
    tx_all = pool.withdrawAll(poolTokens, [0 for _ in pool_tokens], {'from': berg})
    withdrawAllAmount = tx_all.return_value
    
    expectedAmounts = pool_utils.compute_equal_withdrawal(poolTokens, weights, poolBalances, pool.totalSupply() + poolTokens, amp, 0)
    
    for allAmount, poolBalance, expectedAmount in zip(withdrawAllAmount, poolBalances, expectedAmounts):
        if allAmount > expectedAmount:
            assert allAmount <= int(expectedAmount * (1+1e-10) + 1)  # If more is returned, it needs to be almost insignificant. (and there is a deposit fee.)
        assert int(expectedAmount * 99/100) <= allAmount
    

# This function compares the output difference between withdrawAll and withdrawMixed
@example(unbalance_percentage=8000, withdrawal_percentage=9000)
@given(unbalance_percentage=strategy("uint256", min_value=100, max_value=10000), withdrawal_percentage=strategy("uint256", min_value=100, max_value=10000))
@pytest.mark.no_call_coverage
def test_compare_withdrawall_and_withdrawmixed(pool, pool_tokens, berg, deployer, unbalance_percentage, withdrawal_percentage):
    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")
    unbalance_percentage /= 10000
    withdrawal_percentage /= 10000
    
    poolBalances = [token.balanceOf(pool) for token in pool_tokens]
    swap_amount = int(poolBalances[0] * unbalance_percentage)
    poolTokens = int(pool.totalSupply() * withdrawal_percentage)
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})
    pool.transfer(berg, poolTokens, {'from': deployer})
    
    tx_swap = pool.localSwap(
         pool_tokens[0], pool_tokens[1], swap_amount, 0, {'from': berg})
    
    tx_all = pool.withdrawAll(poolTokens, [0 for _ in pool_tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    all_swappool_after_balances = [token.balanceOf(pool) for token in pool_tokens]
    chain.undo()
    
    try: 
        tx_mixed = pool.withdrawMixed(poolTokens, [int(10**18/(len(pool_tokens) - i)) for i in range(len(pool_tokens))], [0 for _ in pool_tokens], {'from': berg})
    except VirtualMachineError:
        assert unbalance_percentage + withdrawal_percentage > 1
        # TODO: Handle
        return
    
    withdrawMixedAmount = tx_mixed.return_value
    
    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        if mixedAmount > allAmount:
            assert  mixedAmount <= int(allAmount * (1+1e-10))  # If more is returned, it needs to be almost insignificant. (and there is a deposit fee.)
        assert int(allAmount * 99 / 100) <= mixedAmount
    
    
    
    
    
    