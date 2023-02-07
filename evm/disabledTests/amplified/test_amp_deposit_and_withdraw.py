import brownie
import pytest

from utils.common import SwapPoolInfo
from utils.swap_utils import run_amp_swap
from utils.deposit_utils import run_amp_deposit, run_amp_withdraw


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# Setup testing environment *****************************************************************************************************

# Define accounts
@pytest.fixture(scope="module")
def deployer(accounts):
    yield accounts[1]

@pytest.fixture(scope="module")
def depositor(accounts):
    yield accounts[2]


# Define and init swap pools
amplification = 2**62
depositValues = [1000 * 10**18, 1000 * 10**18, 1000 * 10**6, 1000 * 10**18]   # Values correspond to the different tokens from the 'tokens' fixture

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

@pytest.fixture(scope="module")
def swappool2_info(deploy_swappool, tokens, deployer):

    # Swap pool params
    tokens        = [tokens[3]]
    balances      = [depositValues[3]]
    weights       = [1]
    name          = "Pool 2"
    symbol        = "P2"

    swappool = deploy_swappool(tokens, balances, amplification, name, symbol, weights, deployer)

    yield SwapPoolInfo(swappool, tokens, balances, weights, amplification, 0, 0, name, symbol, deployer)

@pytest.fixture(scope="module")
def swappool_group(chainId, swappool1_info, swappool2_info, deployer):
    """
        This fixture is used to setup the swappool group. Tests don't need to use it in any manner.
    """

    # Create connections between pools
    swappool1_info.swappool.setConnection(
        chainId,
        brownie.convert.to_bytes(swappool2_info.swappool.address.replace("0x", "")),
        True,
        {"from": deployer},
    )
    swappool2_info.swappool.setConnection(
        chainId,
        brownie.convert.to_bytes(swappool1_info.swappool.address.replace("0x", "")),
        True,
        {"from": deployer},
    )

    yield [swappool1_info, swappool2_info]



# Tests *************************************************************************************************************************

def test_deposit_and_withdraw(
    swappool1_info,
    depositor,
    gov,
    fn_isolation
):
    sp               = swappool1_info.swappool
    tokens           = swappool1_info.tokens
    deposit_balances = swappool1_info.init_token_balances

    initial_total_supply = sp.totalSupply()


    # 1. Deposit as many assets as there are currently in the pool (i.e. duplicate asset balances).
    deposit_amount = initial_total_supply
    run_amp_deposit(
        amount        = deposit_amount,
        swappool_info = swappool1_info,
        depositor     = depositor,
        gov           = gov
    )

    # Check that the deposit was correct. The pool token supply should have doubled
    assert sp.balanceOf(depositor) == initial_total_supply
    assert sp.totalSupply()        == 2 * initial_total_supply

    # Check that all tokens were moved correctly.
    for token, balance in zip(tokens, deposit_balances):
        assert token.balanceOf(sp) == 2 * balance


    # 2. Withdraw all the previously deposited assets
    withdraw_amount = deposit_amount
    run_amp_withdraw(
        amount        = withdraw_amount,
        swappool_info = swappool1_info,
        withdrawer    = depositor
    )

    # Check that the withdrawal was correct. The pool token supply should have returned to initial_total_supply
    assert sp.balanceOf(depositor) == 0
    assert sp.totalSupply()        == initial_total_supply

    # Check that all tokens were moved correctly.
    for token, balance in zip(tokens, deposit_balances):
        assert token.balanceOf(sp) == balance


def test_swap_and_deposit_and_withdraw(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):
    """
        Tests the deposit and withdraw functiona for the case in which At != A0.
    """

    # Deposit as many assets as there are currently in the pool for each asset (i.e. duplicate asset balances).
    sp               = swappool1_info.swappool
    tokens           = swappool1_info.tokens
    deposit_balances = swappool1_info.init_token_balances

    initial_total_supply = sp.totalSupply()


    # 1. Swap
    swap_amount = 1e18
    run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = gov,
        to_swapper         = gov,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov
    )
    assert tokens[0].balanceOf(sp) == deposit_balances[0] + swap_amount


    # 2. Deposit
    deposit_amount = initial_total_supply
    run_amp_deposit(
        amount        = deposit_amount,
        swappool_info = swappool1_info,
        depositor     = depositor,
        gov           = gov
    )

    # Check that the deposit was correct.  # Since the user deposits the same amount as was within the pool, the pool token supply should have doubled
    assert sp.balanceOf(depositor) == initial_total_supply
    assert sp.totalSupply()        == 2 * initial_total_supply

    # Check that all tokens were moved correctly.
    for token, balance in zip(tokens, deposit_balances):

        if token == tokens[0]:
            # The deposit balance is computed as the proportion of new pool tokens (deposit_amount/initial_total_supply) of the current asset balance (balance + swap_amount)
            assert token.balanceOf(sp) == 2 * (balance + swap_amount)
        else:
            assert token.balanceOf(sp) == 2 * balance


    # 3. Withdraw all the previously deposited assets
    withdraw_amount = deposit_amount
    run_amp_withdraw(
        amount        = withdraw_amount,
        swappool_info = swappool1_info,
        withdrawer    = depositor
    )

    # Check that the withdrawal was correct. The pool token supply should have returned to initial_total_supply
    assert sp.balanceOf(depositor) == 0
    assert sp.totalSupply()        == initial_total_supply

    # Check that all tokens were moved correctly.
    for token, balance in zip(tokens, deposit_balances):

        if token == tokens[0]:
            # The deposit balance is computed as the proportion of new pool tokens (deposit_amount/initial_total_supply) of the current asset balance (balance + swap_amount)
            assert token.balanceOf(sp) == balance + swap_amount
        else:
            assert token.balanceOf(sp) == balance


def test_deposit_zero_amount(
    swappool1_info,
    gov,
    depositor,
    fn_isolation
):

    sp                   = swappool1_info.swappool
    initial_total_supply = sp.totalSupply()
    deposit_amount       = 0

    run_amp_deposit(
        amount        = deposit_amount,
        swappool_info = swappool1_info,
        depositor     = depositor,
        gov           = gov
    )

    assert sp.balanceOf(depositor) == 0
    assert sp.totalSupply()        == initial_total_supply



def test_deposit_without_enough_assets(
    swappool1_info,
    depositor,
    fn_isolation
):

    initial_total_supply = swappool1_info.swappool.totalSupply()
    deposit_amount       = initial_total_supply

    with brownie.reverts():
        # Note that the 'depositor' does not have any tokens, and that 'gov' is not passed to the 'run_amp_deposit' function.
        run_amp_deposit(
            amount        = deposit_amount,
            swappool_info = swappool1_info,
            depositor     = depositor
        )


def test_withdraw_zero_amount(
    swappool1_info,
    depositor,
    fn_isolation
):

    sp                   = swappool1_info.swappool
    initial_total_supply = sp.totalSupply()
    withdraw_amount      = 0

    run_amp_withdraw(
        amount        = withdraw_amount,
        swappool_info = swappool1_info,
        withdrawer    = depositor
    )

    assert sp.balanceOf(depositor) == 0
    assert sp.totalSupply()        == initial_total_supply



def test_withdraw_without_enough_pool_tokens(
    swappool1_info,
    depositor,
    fn_isolation
):

    initial_total_supply = swappool1_info.swappool.totalSupply()
    withdraw_amount      = initial_total_supply

    with brownie.reverts():
        # Note that the 'withdrawer' does not have any pool tokens.
        run_amp_withdraw(
            amount        = withdraw_amount,
            swappool_info = swappool1_info,
            withdrawer    = depositor
        )
