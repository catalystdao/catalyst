import brownie
import pytest
import brownie
from brownie import chain
from brownie.test import given, strategy

from utils.common import SwapPoolInfo
from utils.swap_utils import run_amp_local_swap
from utils.utils import assert_relative_error

@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# Setup testing environment *****************************************************************************************************

# Define accounts
@pytest.fixture(scope="module")
def deployer(accounts):
    yield accounts[1]

@pytest.fixture(scope="module")
def swapper(accounts):
    yield accounts[3]


# Define and init swap pools
amplification = 2**62
depositValues = [10 * 10**18, 1000 * 10**18, 1000 * 10**6, 1000 * 10**18]   # Values correspond to the different tokens from the 'tokens' fixture

@pytest.fixture(scope="module")
def swappool1_info(deploy_swappool, tokens, deployer):

    # Swap pool params
    tokens        = tokens[:2]
    balances      = depositValues[:2]
    weights       = [1, 1]
    name          = "Pool 1"
    symbol        = "P1"

    swappool = deploy_swappool(tokens, balances, amplification, name, symbol, weights, deployer)

    yield SwapPoolInfo(swappool, tokens, balances, weights, amplification, 0, 0, name, symbol, deployer)



# Tests *************************************************************************************************************************

#TODO error bounds are larger than those for the non-amp swap pool

@given(swap_amount=strategy("uint256", min_value=1e18, max_value=depositValues[0]))
def test_local_swap(
    swappool1_info,
    swapper,
    gov,
    swap_amount
):

    sp            = swappool1_info.swappool
    tokens        = swappool1_info.tokens
    init_balances = [token.balanceOf(sp) for token in tokens]

    try:
        swap_result = run_amp_local_swap(
            swap_amount        = swap_amount,
            source_token_index = 0,
            target_token_index = 1,
            swappool_info      = swappool1_info,
            swapper            = swapper,
            approx             = False,
            gov                = gov,
            min_amount         = 0,
            large_error_bound  = 1e-3,
            small_error_bound  = 1e-4
        )
    except AssertionError as ae:
        # The swap_yield relative error may be huge for a very small swap (e.g. swapping 1 (1e-18), receiving 0). Allow these to fail.
        assert "(ERR: LOCAL_SWAP_RETURN_ERROR)" in ae.args[0]   # Make sure that we are only catching the specific assertion in question
        assert swap_amount < 1e10
        chain.revert()
        return


    # Reverse swap
    run_amp_local_swap(
        swap_amount        = swap_result.output,
        source_token_index = 1,
        target_token_index = 0,
        swappool_info      = swappool1_info,
        swapper            = swapper,
        approx             = False,
        gov                = None,      # Make sure the swapper does not get funded with assets for the return swap
        min_amount         = 0,
        large_error_bound  = 1e-3,
        small_error_bound  = 1e-4
    )

    # Make sure the pool asset balances are correct
    for token, init_balance in zip(tokens, init_balances):
        assert_relative_error(token.balanceOf(sp), init_balance, -1e-3, 1e-4)
    
    # Reset state
    chain.revert()



@given(swap_amount=strategy("uint256", max_value=1e18))
def test_local_swap_approx(
    swappool1_info,
    swapper,
    gov,
    swap_amount
):

    sp            = swappool1_info.swappool
    tokens        = swappool1_info.tokens
    init_balances = [token.balanceOf(sp) for token in tokens]

    try:
        swap_result = run_amp_local_swap(
            swap_amount        = swap_amount,
            source_token_index = 0,
            target_token_index = 1,
            swappool_info      = swappool1_info,
            swapper            = swapper,
            approx             = True,
            gov                = gov,
            min_amount         = 0,
            large_error_bound  = 1e-3,
            small_error_bound  = 1e-4
        )
    except AssertionError as ae:
        # The swap_yield relative error may be huge for a very small swap (e.g. swapping 1 (1e-18), receiving 0). Allow these to fail.
        assert "(ERR: LOCAL_SWAP_RETURN_ERROR)" in ae.args[0]   # Make sure that we are only catching the specific assertion in question
        assert swap_amount < 1e10
        chain.revert()
        return

    # Reverse swap
    run_amp_local_swap(
        swap_amount        = swap_result.output,
        source_token_index = 1,
        target_token_index = 0,
        swappool_info      = swappool1_info,
        swapper            = swapper,
        approx             = True,
        gov                = None,      # Make sure the swapper does not get funded with assets for the return swap
        min_amount         = 0,
        large_error_bound  = 1e-3,
        small_error_bound  = 1e-4
    )

    # Make sure the pool asset balances are correct
    for token, init_balance in zip(tokens, init_balances):
        assert_relative_error(token.balanceOf(sp), init_balance, -1e-3, 1e-4)



def test_local_swap_with_self(
    swappool1_info,
    swapper,
    gov,
    fn_isolation
):

    # TODO should we disallow swapping a token for itself? The current implementation 'caches' the balance of the output asset 
    # at the beginning of the swap. This results in less assets being returned than were originally provided. This is not a 
    # security problem for Catalyst, but may cause users to lose funds because of erroneous usage of the pools. 

    sp    = swappool1_info.swappool
    token = swappool1_info.tokens[0]

    swap_amount = 10**18

    token.transfer(swapper, swap_amount, {"from": gov})
    token.approve(sp, swap_amount, {"from": swapper})

    # Not using the 'run_local_swap' helper function, as its checks fail when the input and output assets are the same
    tx = sp.localSwap(
        token,
        token,
        swap_amount,
        0,
        {"from": swapper}
    )

    assert tx.events['LocalSwap'][0]['toAmount'] <= swap_amount



def test_local_swap_min_amount(
    swappool1_info,
    swapper,
    gov,
    fn_isolation
):

    swap_amount = 1e18
    min_amount  = 2**256-1
    with brownie.reverts():
        run_amp_local_swap(
            swap_amount        = swap_amount,
            source_token_index = 0,
            target_token_index = 1,
            swappool_info      = swappool1_info,
            swapper            = swapper,
            approx             = False,
            gov                = gov,
            min_amount         = min_amount
        )



def test_local_swap_no_funds(
    swappool1_info,
    swapper,
    fn_isolation
):

    swap_amount = 1e18
    with brownie.reverts():
        run_amp_local_swap(
            swap_amount        = swap_amount,
            source_token_index = 0,
            target_token_index = 1,
            swappool_info      = swappool1_info,
            swapper            = swapper,
            approx             = False,
            gov                = None,  # Set gov to None to make sure the swapper does not have funds
            min_amount         = 0
        )



def test_local_swap_no_approx_method(
    swappool1_info,
    swapper,
    gov,
    fn_isolation
):
    """
        For coverage purposes: make sure the 'localSwap' overload that excludes the 'approx' argument gets invoked.
        TODO Make sure that the overload correctly invokes the full 'localSwap' method with approx=False.
        TODO As of brownie 1.19.1 (on WSL Ubuntu), this invokation is not shown on the 'subcalls' property of the 'transaction' object.
        TODO No other (simple) means of testing this has been found. 
    """

    swap_amount = 1e18
    swap_result = run_amp_local_swap(
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 1,
        swappool_info      = swappool1_info,
        swapper            = swapper,
        approx             = None,
        gov                = gov,
        min_amount         = 0,
        large_error_bound  = 1e-3,
        small_error_bound  = 1e-4
    )



def test_local_swap_with_invalid_input_asset(
    swappool1_info,
    swapper,
    gov,
    tokens,
    fn_isolation
):

    sp = swappool1_info.swappool

    token_in  = tokens[3]   # Not in swap pool
    token_out = swappool1_info.tokens[0]

    swap_amount = 10**18

    token_in.transfer(swapper, swap_amount, {"from": gov})
    token_in.approve(sp, swap_amount, {"from": swapper})

    # Send maliciously assets (the input token) to the pool to avoid getting division by 0 error
    token_in.transfer(sp, 10**18, {"from": gov})

    # Not using the 'run_local_swap' helper function as its checks fail in these conditions
    tx = sp.localSwap(
        token_in,
        token_out,
        swap_amount,
        0,
        {"from": swapper}
    )

    assert tx.events['LocalSwap'][0]['toAmount'] == 0
