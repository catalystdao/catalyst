import pytest
import brownie
from brownie import chain

from utils.common import SwapPoolInfo
from utils.utils import relative_error
from utils.swap_utils import run_amp_swap
from utils.liquidity_swap_utils import run_amp_liquidity_swap
from utils.deposit_utils import run_withdraw

@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


depositValues = [1000 * 10**18, 1000 * 10**18, 1000 * 10**18]
amplification = 2**62

# Define accounts
@pytest.fixture(scope="module")
def deployer(accounts):
    yield accounts[1]

@pytest.fixture(scope="module")
def depositor(accounts):
    yield accounts[2]


# Create pools

@pytest.fixture(scope="module")
def token4(create_token):
    yield create_token("four", "TTTT", decimal=18)

@pytest.fixture(scope="module")
def swappool1_info(deploy_swappool, token1, token2, deployer):

    # Swap pool params
    tokens        = [token1, token2]
    balances      = depositValues[0:2]
    weights       = [1, 1]
    name          = "Pool 1"
    symbol        = "P1"

    swappool = deploy_swappool(tokens, balances, amplification, name, symbol, weights, deployer)

    yield SwapPoolInfo(swappool, tokens, balances, weights, amplification, 0, 0, name, symbol, deployer)


@pytest.fixture(scope="module")
def swappool2_info(deploy_swappool, token4, deployer):

    # Swap pool params
    tokens        = [token4]
    balances      = [depositValues[2]]
    weights       = [1]
    name          = "Pool 2"
    symbol        = "P2"

    swappool = deploy_swappool(tokens, balances, amplification, name, symbol, weights, deployer)

    yield SwapPoolInfo(swappool, tokens, balances, weights, amplification, 0, 0, name, symbol, deployer)


def test_connect_pools(chainId, swappool1_info, swappool2_info, deployer):

    # Create connections between pools
    swappool1_info.swappool.createConnection(
        chainId,
        brownie.convert.to_bytes(swappool2_info.swappool.address.replace("0x", "")),
        True,
        {"from": deployer},
    )
    swappool2_info.swappool.createConnection(
        chainId,
        brownie.convert.to_bytes(swappool1_info.swappool.address.replace("0x", "")),
        True,
        {"from": deployer},
    )



# Add a new depositor, whom will be the one liquidity swapping
def test_add_depositor(swappool1_info, gov, depositor):

    # Deposit as many assets as there are currently in the pool for each asset (i.e. duplicate asset balances).
    swappool1        = swappool1_info.swappool
    tokens           = swappool1_info.tokens
    deposit_balances = swappool1_info.init_token_balances

    # Transfer tokens from token minter (gov) to the user (depositor) and approve the swap pool to access those tokens.
    for token, balance in zip(tokens, deposit_balances):
        token.transfer(depositor, balance, {"from": gov})
        token.approve(swappool1, balance, {"from": depositor})

    initial_total_supply = swappool1.totalSupply()

    # Deposit
    swappool1.depositMixed(deposit_balances, initial_total_supply, {"from": depositor})

    # Check that the deposit was correct.  # Since the user deposits the same amount as was within the pool, the pool token supply should have doubled
    assert swappool1.balanceOf(depositor) == initial_total_supply
    assert swappool1.totalSupply()        == 2 * initial_total_supply

    # Check that all tokens were moved correctly.
    for token, balance in zip(tokens, deposit_balances):
        assert token.balanceOf(depositor) == 0
        assert token.balanceOf(swappool1) == 2 * balance
        assert swappool1._balance0(token) == 2 * balance    # This is true, as the balance0 of the asset is initialized to be the same as the initial balance deposited when creating the pool (and hence it gets doubled)

    chain.snapshot()



def test_liquidity_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):
    # Make sure test is setup correctly.
    assert swappool1_info.swappool.totalSupply() == 2 * 2**64
    assert swappool2_info.swappool.totalSupply() == 2**64

    assert swappool1_info.swappool.balanceOf(depositor) == 2**64
    assert swappool2_info.swappool.balanceOf(depositor) == 0

    assert swappool1_info.tokens[0].balanceOf(depositor) == 0   # Required for later check
    assert swappool1_info.tokens[1].balanceOf(depositor) == 0   # Required for later check

    # Swap liquidity from pool 1 to pool 2
    run_amp_liquidity_swap(
        chainId,
        2**62,
        swappool1_info,
        swappool2_info,
        False,
        False,
        depositor,
        ibcemulator,
        gov
    )

    assert swappool1_info.swappool.balanceOf(depositor) == 2**64 - 2**62
    assert swappool2_info.swappool.balanceOf(depositor) != 0

    # Swap from pool 2 to pool 1
    swap_amount = int(swappool2_info.tokens[0].balanceOf(swappool2_info.swappool) * 0.2)

    run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = gov,
        to_swapper         = gov,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov
    )

    # Withdraw everything
    run_withdraw(
        amount        = None, # i.e. everything
        swappool_info = swappool1_info,
        withdrawer    = depositor
    )

    assert swappool1_info.swappool.balanceOf(depositor) == 0
    assert swappool2_info.swappool.balanceOf(depositor) != 0

    # Make sure depositor receives less tokens than initially deposited, as some of it's liquidity has been transferred to another pool
    assert relative_error(swappool1_info.tokens[0].balanceOf(depositor), depositValues[0]) < -0.1   # 0.1 is arbitrary; just to make sure the decrease is significant and not a rounding error
    assert relative_error(swappool1_info.tokens[1].balanceOf(depositor), depositValues[1]) < -0.1   # 0.1 is arbitrary; just to make sure the decrease is significant and not a rounding error



def test_liquidity_swap_with_self(
    chainId,
    swappool1_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):
    init_sp_pool_token_supply         = swappool1_info.swappool.totalSupply()
    init_depositor_pool_token_balance = swappool1_info.swappool.balanceOf(depositor)

    # Make sure test is setup correctly.
    assert init_sp_pool_token_supply == 2 * 2**64
    assert init_depositor_pool_token_balance == 2**64
    assert swappool1_info.tokens[0].balanceOf(depositor) == 0   # Required for later check

    # Swap liquidity from pool 1 to itself
    swap_amount = 2**62
    run_amp_liquidity_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool1_info,
        approx_out         = False,
        approx_in          = False,
        depositor          = depositor,
        ibcemulator        = ibcemulator,
        gov                = gov
    )

    # Check swappool supply
    assert -0.01 < relative_error(swappool1_info.swappool.totalSupply(), init_sp_pool_token_supply) <= 0

    # Check swapper pool tokens
    assert -0.01 < relative_error(swappool1_info.swappool.balanceOf(depositor), init_depositor_pool_token_balance) <= 0



def test_approx_liquidity_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):
    # Make sure test is setup correctly.
    assert swappool1_info.swappool.totalSupply() == 2 * 2**64
    assert swappool2_info.swappool.totalSupply() == 2**64

    assert swappool1_info.swappool.balanceOf(depositor) == 2**64
    assert swappool2_info.swappool.balanceOf(depositor) == 0

    assert swappool1_info.tokens[0].balanceOf(depositor) == 0   # Required for later check
    assert swappool1_info.tokens[1].balanceOf(depositor) == 0   # Required for later check

    run_amp_liquidity_swap(
        chainId,
        2**55,
        swappool1_info,
        swappool2_info,
        True,
        True,
        depositor,
        ibcemulator,
        gov
    )

    assert swappool1_info.swappool.balanceOf(depositor) == 2**64 - 2**55
    assert swappool2_info.swappool.balanceOf(depositor) != 0

    # Swap from pool 2 to pool 1
    swap_amount = int(swappool2_info.tokens[0].balanceOf(swappool2_info.swappool) * 0.2)

    run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = gov,
        to_swapper         = gov,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov
    )

    # Withdraw everything
    run_withdraw(
        amount        = None, # i.e. everything
        swappool_info = swappool1_info,
        withdrawer    = depositor
    )

    assert swappool1_info.swappool.balanceOf(depositor) == 0
    assert swappool2_info.swappool.balanceOf(depositor) != 0

    # Make sure depositor receives less tokens than initially deposited, as some of it's liquidity has been transferred to another pool
    assert relative_error(swappool1_info.tokens[0].balanceOf(depositor), depositValues[0]) < -0.0001   # 0.0001 is arbitrary; just to make sure the decrease is significant and not a rounding error
    assert relative_error(swappool1_info.tokens[1].balanceOf(depositor), depositValues[1]) < -0.0001   # 0.0001 is arbitrary; just to make sure the decrease is significant and not a rounding error

