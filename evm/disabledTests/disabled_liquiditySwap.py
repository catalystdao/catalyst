import pytest
import brownie
from brownie import chain, ZERO_ADDRESS

from utils.common import SwapPoolInfo
from utils.utils import assert_relative_error, relative_error
from utils.swap_utils import run_swap, decodePayload
from utils.liquidity_swap_utils import run_liquidity_swap
from utils.deposit_utils import run_deposit, run_withdraw


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

@pytest.fixture(scope="module")
def hacker(accounts):
    yield accounts[5]


# Define and init swap pools
amplification = 2**64
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

@pytest.fixture(scope="module")
def swappool2_info(deploy_swappool, tokens, deployer):

    # Swap pool params
    tokens        = [tokens[2]]
    balances      = [depositValues[2]]
    weights       = [1]
    name          = "Pool 2"
    symbol        = "P2"

    swappool = deploy_swappool(tokens, balances, amplification, name, symbol, weights, deployer)

    yield SwapPoolInfo(swappool, tokens, balances, weights, amplification, 0, 0, name, symbol, deployer)

@pytest.fixture(scope="module", autouse=True)
def swappool_group(chainId, swappool1_info, swappool2_info, deployer):
    """
        This fixture is used to setup the swappool group. Tests don't need to use it in any manner.
    """

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

    yield [swappool1_info, swappool2_info]



# Add a new depositor, whom will be the one liquidity swapping
@pytest.fixture(scope="module", autouse=True)
def depositor_init(swappool1_info, swappool2_info, gov, depositor):
    """
        This fixture is used to add a depositor to both pools. Tests don't need to use it in any manner.
    """

    # Swap pool 1: deposit as many assets as there are currently in the pool for each asset (i.e. duplicate asset balances).
    run_deposit(
        amount        = swappool1_info.swappool.totalSupply(),
        swappool_info = swappool1_info,
        depositor     = depositor,
        gov           = gov
    )

    # Swap pool 2: deposit 2/3 of the assets as there are currently in the pool for each asset.
    run_deposit(
        amount        = int(2/3 * swappool2_info.swappool.totalSupply()),
        swappool_info = swappool2_info,
        depositor     = depositor,
        gov           = gov
    )



def test_liquidity_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    init_sp1_supply = sp1.totalSupply()
    init_sp2_supply = sp2.totalSupply()

    init_depositor_sp1_balance = sp1.balanceOf(depositor)
    init_depositor_sp2_balance = sp2.balanceOf(depositor)


    # 1. Swap liquidity from pool 1 to pool 2
    initial_swap_amount = 2**62
    result_1 = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = initial_swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )
    liquidity_swap_yield = result_1.run_finish_liquidity_swap_result.output

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance - initial_swap_amount
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance + liquidity_swap_yield

    # # TODO security limit check

    # 2. Reverse swap half-way (that is, use only half of the yield of the previous operation)
    initial_reverse_swap_amount = int(liquidity_swap_yield/2)
    result_2 = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = initial_reverse_swap_amount,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )
    liquidity_swap_yield_2 = result_2.run_finish_liquidity_swap_result.output

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance - initial_swap_amount + liquidity_swap_yield_2
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance + liquidity_swap_yield - initial_reverse_swap_amount

    # # TODO security limit check

    # 3. Reverse swap again (swap again using the full 'swap_yield' balance, hence swapper2 exceeds the liquidity it recieved with the original forward swap.)
    result_3 = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = liquidity_swap_yield,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )
    liquidity_swap_yield_3 = result_3.run_finish_liquidity_swap_result.output

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance - initial_swap_amount + liquidity_swap_yield_2 + liquidity_swap_yield_3
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance + liquidity_swap_yield - initial_reverse_swap_amount - liquidity_swap_yield


    # 4. Rebalance pools
    rebalance_amount = sp1.totalSupply() - init_sp1_supply
    assert rebalance_amount == -(initial_swap_amount - liquidity_swap_yield_2 - liquidity_swap_yield_3)
    run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = rebalance_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )

    assert sp1.totalSupply() == init_sp1_supply
    assert_relative_error(sp2.totalSupply(), init_sp2_supply, -1e7, 1e7)

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance
    assert_relative_error(sp2.balanceOf(depositor), init_depositor_sp2_balance, -1e7, 1e7)



def test_approx_liquidity_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    init_sp1_supply = sp1.totalSupply()
    init_sp2_supply = sp2.totalSupply()

    init_depositor_sp1_balance = sp1.balanceOf(depositor)
    init_depositor_sp2_balance = sp2.balanceOf(depositor)


    # 1. Swap liquidity from pool 1 to pool 2
    initial_swap_amount = 2**50
    result_1 = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = initial_swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = True,
        approx_in          = True,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )
    liquidity_swap_yield = result_1.run_finish_liquidity_swap_result.output

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance - initial_swap_amount
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance + liquidity_swap_yield

    # # TODO security limit check

    # 2. Reverse swap half-way (that is, use only half of the yield of the previous operation)
    initial_reverse_swap_amount = int(liquidity_swap_yield/2)
    result_2 = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = initial_reverse_swap_amount,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = True,
        approx_in          = True,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )
    liquidity_swap_yield_2 = result_2.run_finish_liquidity_swap_result.output

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance - initial_swap_amount + liquidity_swap_yield_2
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance + liquidity_swap_yield - initial_reverse_swap_amount

    # # TODO security limit check

    # 3. Reverse swap again (swap again using the full 'swap_yield' balance, hence swapper2 exceeds the liquidity it recieved with the original forward swap.)
    result_3 = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = liquidity_swap_yield,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = True,
        approx_in          = True,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )
    liquidity_swap_yield_3 = result_3.run_finish_liquidity_swap_result.output

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance - initial_swap_amount + liquidity_swap_yield_2 + liquidity_swap_yield_3
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance + liquidity_swap_yield - initial_reverse_swap_amount - liquidity_swap_yield


    # 4. Rebalance pools
    rebalance_amount = sp1.totalSupply() - 2*2**64
    assert rebalance_amount == -(initial_swap_amount - liquidity_swap_yield_2 - liquidity_swap_yield_3)
    run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = rebalance_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = True,
        approx_in          = True,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )

    assert sp1.totalSupply() == init_sp1_supply
    assert_relative_error(sp2.totalSupply(), init_sp2_supply, -1e7, 1e7)

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance
    assert_relative_error(sp2.balanceOf(depositor), init_depositor_sp2_balance, -1e7, 1e7)



def test_liquidity_swap_with_swap_and_withdraw(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    init_sp1_supply = sp1.totalSupply()
    init_sp2_supply = sp2.totalSupply()

    init_depositor_sp1_balance = sp1.balanceOf(depositor)
    init_depositor_sp2_balance = sp2.balanceOf(depositor)

    # Swap liquidity from pool 1 to pool 2
    initial_swap_amount = 2**62
    run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = initial_swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov
    )

    # Swap from pool 2 to pool 1
    swap_amount = int(swappool2_info.tokens[0].balanceOf(swappool2_info.swappool) * 0.2)
    run_swap(
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

    # Withdraw everything from pool 1
    run_withdraw(
        amount        = None, # i.e. everything
        swappool_info = swappool1_info,
        withdrawer    = depositor
    )

    assert swappool1_info.swappool.balanceOf(depositor) == 0
    assert swappool2_info.swappool.balanceOf(depositor) > init_depositor_sp2_balance

    # Make sure depositor receives less tokens than initially deposited, as some of it's liquidity has been transferred to another pool
    # Note that the tests are configured such that the depositor intially deposits on pool 1 as much assets as there are initially upon pool setup (i.e. depositValues)
    assert relative_error(swappool1_info.tokens[0].balanceOf(depositor), depositValues[0]) < -0.1   # 0.1 is arbitrary; just to make sure the decrease is significant and not a rounding error
    assert relative_error(swappool1_info.tokens[1].balanceOf(depositor), depositValues[1]) < -0.1   # 0.1 is arbitrary; just to make sure the decrease is significant and not a rounding error



def test_timed_out_liquidity_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    gov,
    depositor,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    init_sp1_supply = sp1.totalSupply()
    init_sp2_supply = sp2.totalSupply()

    init_depositor_sp1_balance = sp1.balanceOf(depositor)
    init_depositor_sp2_balance = sp2.balanceOf(depositor)

    # 1. Swap
    swap_amount = 2**62
    result = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov,
        force_timeout      = True
    )

    assert sp1.totalSupply() == init_sp1_supply
    assert sp2.totalSupply() == init_sp2_supply

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance



def test_liquidity_swap_too_large(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    gov,
    depositor,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    # Increase depositor pool token share to be able to exceed the security limit
    run_deposit(
        amount        = sp1.totalSupply(),
        swappool_info = swappool1_info,
        depositor     = depositor,
        gov           = gov
    )


    init_sp1_supply = sp1.totalSupply()
    init_sp2_supply = sp2.totalSupply()

    init_depositor_sp1_balance = sp1.balanceOf(depositor)
    init_depositor_sp2_balance = sp2.balanceOf(depositor)

    # 1. Swap
    swap_amount = int(init_sp1_supply/2) + 1
    result = run_liquidity_swap(
        chainId             = chainId,
        swap_amount         = swap_amount,
        from_swappool_info  = swappool1_info,
        to_swappool_info    = swappool2_info,
        from_swapper        = depositor,
        to_swapper          = depositor,
        approx_out          = False,
        approx_in           = False,
        ibcemulator         = ibcemulator,
        ibc_gov             = gov,
        allow_target_revert = True
    )

    # Make sure the operation reverted
    run_finish_liquidity_swap_result = result.run_finish_liquidity_swap_result
    assert run_finish_liquidity_swap_result.revert_exception is not None
    assert run_finish_liquidity_swap_result.revert_exception.args[0].args[0]['message'] == \
        'VM Exception while processing transaction: revert Swap exceeds maximum swap amount'

    # Check swapper balances

    assert sp1.totalSupply() == init_sp1_supply
    assert sp2.totalSupply() == init_sp2_supply

    assert sp1.balanceOf(depositor) == init_depositor_sp1_balance
    assert sp2.balanceOf(depositor) == init_depositor_sp2_balance



def test_direct_swap_from_liquidity_units_invocation(
    swappool1_info,
    hacker,
    fn_isolation
):
    sp = swappool1_info.swappool

    # Try to directly invoke swapFromUnits
    with brownie.reverts(): # TODO dev msg
        sp.inLiquidity(
            hacker,
            2**60,
            0,
            False,
            ZERO_ADDRESS,   # message hash, anything will do
            {"from": hacker}
        )



def test_direct_escrow_ack_timeout_invocation(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    hacker,
    ibcemulator,
    gov,
    fn_isolation
):
    sp1 = swappool1_info.swappool

    # Create an escrow
    swap_amount = 2**62
    swap_result = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov,
        finish_swap        = False
    )

    message_hash      = swap_result.tx_swap_to_liquidity_units.events['SwapToLiquidityUnits'][0]['messageHash']
    transferred_units = swap_result.tx_swap_to_liquidity_units.events['SwapToLiquidityUnits']['output']
    escrowAmount      = decodePayload(swap_result.tx_swap_to_liquidity_units.events["IncomingPacket"]["packet"][3])["_escrowAmount"]

    # Try to directly invoke ack
    with brownie.reverts(): # TODO dev msg
        sp1.releaseLiquidityEscrowACK(message_hash, transferred_units, escrowAmount, {"from": hacker})

    # Try to directly invoke timeout
    with brownie.reverts(): # TODO dev msg
        sp1.releaseLiquidityEscrowTIMEOUT(message_hash, transferred_units, escrowAmount, {"from": hacker})



def test_swap_finish_with_manipulated_packet(
    chainId,
    swappool1_info,
    swappool2_info,
    depositor,
    ibcemulator,
    gov,
    fn_isolation
):
    sp1 = swappool1_info.swappool

    # Create an escrow
    swap_amount = 2**62
    swap_result = run_liquidity_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = depositor,
        to_swapper         = depositor,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        ibc_gov            = gov,
        finish_swap        = False
    )

    # Try to finish the swap
    ibc_target_contract = swap_result.tx_swap_to_liquidity_units.events["IncomingMetadata"]["metadata"][0]
    ibc_packet          = swap_result.tx_swap_to_liquidity_units.events["IncomingPacket"]["packet"]

    # Manipulate 'units' from within the packet
    data = ibc_packet[3]
    increased_units = int.from_bytes(data[97:129], 'big') * 2
    malicious_data = \
        data[0:97]                          + \
        increased_units.to_bytes(32, 'big') + \
        data[129:]

    malicious_ibc_packet = (*ibc_packet[:3], malicious_data, *ibc_packet[4:])

    with brownie.reverts(): # This will revert since there is a check for modified messages.
        ibcemulator.ack(
            ibc_target_contract,
            malicious_ibc_packet,
            {"from": gov},
        )

    # TODO verify security limit doesn't actually change


# TODO swap with self
# def test_liquidity_swap_with_self(
#     chainId,
#     swappool1_info,
#     depositor,
#     ibcemulator,
#     gov,
#     fn_isolation
# ):
#     init_sp_pool_token_supply         = swappool1_info.swappool.totalSupply()
#     init_depositor_pool_token_balance = swappool1_info.swappool.balanceOf(depositor)

#     # Make sure test is setup correctly.
#     assert init_sp_pool_token_supply == 2 * 2**64
#     assert init_depositor_pool_token_balance == 2**64
#     assert swappool1_info.tokens[0].balanceOf(depositor) == 0   # Required for later check

#     # Swap liquidity from pool 1 to itself
#     swap_amount = 2**62
#     run_liquidity_swap(
#         chainId            = chainId,
#         swap_amount        = swap_amount,
#         from_swappool_info = swappool1_info,
#         to_swappool_info   = swappool1_info,
#         from_swapper       = depositor,
#         to_swapper         = depositor,
#         approx_out         = False,
#         approx_in          = False,
#         ibcemulator        = ibcemulator,
#         ibc_gov            = gov
#     )

#     # Check swappool supply
#     assert -0.01 < relative_error(swappool1_info.swappool.totalSupply(), init_sp_pool_token_supply) <= 0

#     # Check swapper pool tokens
#     assert -0.01 < relative_error(swappool1_info.swappool.balanceOf(depositor), init_depositor_pool_token_balance) <= 0

