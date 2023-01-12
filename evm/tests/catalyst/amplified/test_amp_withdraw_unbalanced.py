import pytest
from brownie import chain, reverts
from brownie.test import given, strategy
from brownie.exceptions import VirtualMachineError


# This function compares the output difference between withdrawAll and withdrawMixed
@given(unbalance_percentage=strategy("uint256", min_value=100, max_value=10000), withdrawal_percentage=strategy("uint256", min_value=100, max_value=10000))
def test_withdrawall(swappool_amp, get_pool_tokens, compute_expected_swap_given_U,compute_withdraw_to_U, berg, molly, deployer, unbalance_percentage, withdrawal_percentage):
    unbalance_percentage /= 10000
    withdrawal_percentage /= 10000
    
    tokens = get_pool_tokens(swappool_amp)
    poolBalances = [token.balanceOf(swappool_amp) for token in tokens]
    swap_amount = int(poolBalances[0] * unbalance_percentage)
    poolTokens = int(swappool_amp.totalSupply() * withdrawal_percentage)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool_amp, swap_amount, {'from': berg})
    swappool_amp.transfer(berg, poolTokens, {'from': molly})
    
    tx_all = swappool_amp.withdrawAll(poolTokens, [0 for _ in tokens], {'from': berg})
    withdrawAllAmount = tx_all.return_value
    chain.undo()
    
    U = compute_withdraw_to_U(poolTokens, swappool_amp)
    for allAmount, poolBalance, token in zip(withdrawAllAmount, poolBalances, tokens):
        expectedAmount = compute_expected_swap_given_U(U/len(tokens), token, swappool_amp)
        if expectedAmount > poolBalance:
            expectedAmount = poolBalance
        
        assert allAmount <= expectedAmount
        assert int(expectedAmount * 9/10) <= allAmount
    

# This function compares the output difference between withdrawAll and withdrawMixed
@given(unbalance_percentage=strategy("uint256", min_value=100, max_value=10000), withdrawal_percentage=strategy("uint256", min_value=100, max_value=10000))
def test_compare_withdrawall_and_withdrawmixed(swappool_amp, get_pool_tokens, compute_withdraw_to_U, compute_expected_swap_given_U, berg, molly, deployer, unbalance_percentage, withdrawal_percentage):
    unbalance_percentage /= 10000
    withdrawal_percentage /= 10000
    
    tokens = get_pool_tokens(swappool_amp)
    poolBalances = [token.balanceOf(swappool_amp) for token in tokens]
    swap_amount = int(poolBalances[0] * unbalance_percentage)
    poolTokens = int(swappool_amp.totalSupply() * withdrawal_percentage)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool_amp, swap_amount, {'from': berg})
    swappool_amp.transfer(berg, poolTokens, {'from': molly})
    
    tx_swap = swappool_amp.localswap(
         tokens[0], tokens[1], swap_amount, 0, {'from': berg})
    
    tx_all = swappool_amp.withdrawAll(poolTokens, [0 for _ in tokens], {'from': berg})
    
    withdrawAllAmount = tx_all.return_value
    all_swappool_after_balances = [token.balanceOf(swappool_amp) for token in tokens]
    chain.undo()
    
    emptied_pool = False
    if min(all_swappool_after_balances) == 0:
        tokenIndex = all_swappool_after_balances.index(0)
        toToken = tokens[tokenIndex]
        U = compute_withdraw_to_U(poolTokens, swappool_amp)
        
        try:
            computedOutput = compute_expected_swap_given_U(U/len(tokens), toToken, swappool_amp)
            assert computedOutput >= poolBalances[tokenIndex]
            emptied_pool = True
        except TypeError:
            emptied_pool = True
            
    
    try: 
        tx_mixed = swappool_amp.withdrawMixed(poolTokens, [int(10**18/(len(tokens) - i)) for i in range(len(tokens))], [0 for _ in tokens], {'from': berg})
    except VirtualMachineError:
        assert emptied_pool == True
        return
    
    withdrawMixedAmount = tx_mixed.return_value
    
    for allAmount, mixedAmount in zip(withdrawAllAmount, withdrawMixedAmount):
        assert mixedAmount <= allAmount
        assert int(allAmount * 7 / 10) <= mixedAmount
    
    
    
    
    
    