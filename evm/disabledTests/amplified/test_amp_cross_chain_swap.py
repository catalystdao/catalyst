import pytest
import brownie
from brownie import DummyTargetContract

from utils.common import SwapPoolInfo
from utils.swap_utils import run_amp_swap, decodePayload
from utils.utils import assert_relative_error
from utils.deposit_utils import get_amp_swappool_group_invariant


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# Setup testing environment *****************************************************************************************************

# Define accounts
@pytest.fixture(scope="module")
def deployer(accounts):
    yield accounts[1]

@pytest.fixture(scope="module")
def swapper1(accounts):
    yield accounts[3]

@pytest.fixture(scope="module")
def swapper2(accounts):
    yield accounts[4]

@pytest.fixture(scope="module")
def hacker(accounts):
    yield accounts[5]


@pytest.fixture(scope="module")
def dummy_target_contract(gov):
    yield gov.deploy(DummyTargetContract)


# Define and init swap pools
amplification = 2**60
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
    tokens        = [tokens[3]]
    balances      = [depositValues[3]]
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

# Dev NOTE:
# There is no test for approximate calcaulation of swaps as these do not exists for amp pools (contrary to non-amp pools).
# Full coverage is achieved nonetheless, as even though some 'sendSwap' overloads do take an 'approx' parameter, they do nothing
# else than to call the 'sendSwap' overload that does not include the 'approx' parameter. Given that the swap helpers use the
# overloads that include the 'approx' parameter, all overloads get tested, hence achieving full coverage.

# TODO hypothesis?
def test_successful_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    swapper1,
    swapper2,
    gov,
    fn_isolation
):
    """
        Swap both ways. Reverse swap is done in multiple steps for security limit coverage.
    """

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    token1 = swappool1_info.tokens[0]
    token2 = swappool2_info.tokens[0]

    # 1. Swap
    initial_swap_amount = 10**18
    result_1 = run_amp_swap(
        chainId            = chainId,
        swap_amount        = initial_swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,                       # Note that 'gov' will be providing the assets
        ibc_gov            = gov,
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )
    swap_yield = result_1.run_finish_swap_result.output

    assert sp1.getUnitCapacity() == sp1._maxUnitCapacity()
    assert sp2.getUnitCapacity() < sp2._maxUnitCapacity()
    assert swappool1_info.tokens[0].balanceOf(sp1) == swappool1_info.init_token_balances[0] + initial_swap_amount

    # 2. Reverse swap half-way (that is, use only half of the yield of the previous operation)
    initial_reverse_swap_amount = int(swap_yield/2)
    result_2 = run_amp_swap(
        chainId            = chainId,
        swap_amount        = initial_reverse_swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = swapper2,
        to_swapper         = swapper1,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = None,                      # No extra tokens should be required
        ibc_gov            = gov,
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )
    swap_yield_2 = result_2.run_finish_swap_result.output
    assert swappool1_info.tokens[0].balanceOf(sp1) == swappool1_info.init_token_balances[0] + initial_swap_amount - swap_yield_2

    assert sp1.getUnitCapacity() < sp1._maxUnitCapacity()
    assert sp2.getUnitCapacity() < sp2._maxUnitCapacity()

    # 3. Reverse swap again (swap again using the full 'swap_yield' balance, hence swapper2 exceeds the funds it recieved with the original forward swap. These funds are provided by 'gov')
    result_3 = run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_yield,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool2_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = swapper2,
        to_swapper         = swapper1,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,                       # Since the previous 'reverse' swap used half of the original yield, 'gov' the missing tokens
        ibc_gov            = gov,
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )
    swap_yield_3 = result_3.run_finish_swap_result.output
    assert swappool1_info.tokens[0].balanceOf(sp1) == swappool1_info.init_token_balances[0] + initial_swap_amount - swap_yield_2 - swap_yield_3

    assert sp1.getUnitCapacity() < sp1._maxUnitCapacity()
    assert sp2.getUnitCapacity() == sp2._maxUnitCapacity()


    # 4. Rebalance pools
    rebalance_amount = swappool1_info.init_token_balances[0] - swappool1_info.tokens[0].balanceOf(sp1)
    assert rebalance_amount == -(initial_swap_amount - swap_yield_2 - swap_yield_3)
    run_amp_swap(
        chainId            = chainId,
        swap_amount        = rebalance_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = None,  # No extra tokens should be required
        ibc_gov            = gov,
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )

    assert token1.balanceOf(sp1) == swappool1_info.init_token_balances[0]
    assert_relative_error(token2.balanceOf(sp2), swappool2_info.init_token_balances[0], -1e7, 1e7)  # The pools have been rebalanced w.r.t. the balance of sp1, hence sp2 may deviate slightly from the original balance due to numerical errors

    # After rebalancing, each swapper should only have the assets provided by 'gov' to them
    assert token1.balanceOf(swapper1) == initial_swap_amount
    assert_relative_error(token2.balanceOf(swapper2), initial_reverse_swap_amount, -1e7, 1e7)



def test_swap_with_self(
    chainId,
    swappool1_info,
    ibcemulator,
    swapper1,
    gov,
    fn_isolation
):

    swap_amount = 10**18
    result_1 = run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool1_info,
        from_swapper       = swapper1,
        to_swapper         = swapper1,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,                       # Note that 'gov' will be providing the assets
        ibc_gov            = gov,
        large_error_bound   = 2,
        small_error_bound   = 2
    )
    swap_yield = result_1.run_finish_swap_result.output

    assert swap_yield < swap_amount     # Note that swap_yield is much less than swap_amount since the input assets get escrowed until the ack gets triggered





def test_min_out_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    gov,
    swapper1,
    swapper2,
    fn_isolation
):

    swap_amount = 1e18
    min_out     = 2**256-1
    swap_result = run_amp_swap(
        chainId             = chainId,
        swap_amount         = swap_amount,
        source_token_index  = 0,
        target_token_index  = 0,
        from_swappool_info  = swappool1_info,
        to_swappool_info    = swappool2_info,
        from_swapper        = swapper1,
        to_swapper          = swapper2,
        approx_out          = False,
        approx_in           = False,
        ibcemulator         = ibcemulator,
        token_gov           = gov,
        ibc_gov             = gov,
        min_amount          = min_out,
        allow_target_revert = True
    )

    # Make sure the operation reverted
    finish_swap_result = swap_result.run_finish_swap_result
    assert finish_swap_result.tx_receive_swap is None
    assert finish_swap_result.output is None
    assert finish_swap_result.tx_escrow_timeout is not None

    # Check swapper balances
    assert swappool1_info.tokens[0].balanceOf(swapper1) == swap_amount      # Source swapper recovers the funds
    assert swappool2_info.tokens[0].balanceOf(swapper2) == 0



def test_swap_with_fees(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    swapper1,
    swapper2,
    gov,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    # Setup fees
    pool_fee = 0.1
    gov_fee  = 0.05

    swappool1_info                = SwapPoolInfo(**swappool1_info.__dict__)     # Copy object to not affect other tests (the fees are only present on this test)
    swappool1_info.pool_fee       = pool_fee
    swappool1_info.governance_fee = gov_fee

    swappool2_info                = SwapPoolInfo(**swappool2_info.__dict__)     # Copy object to not affect other tests (the fees are only present on this test)
    swappool2_info.pool_fee       = pool_fee
    swappool2_info.governance_fee = gov_fee

    sp1.setFeeAdministrator(gov, {"from": gov})
    sp1.setPoolFee(int(pool_fee*2**64), {"from": gov})
    sp1.setGovernanceFee(int(gov_fee*2**64), {"from": gov})

    sp2.setFeeAdministrator(gov, {"from": gov})
    sp2.setPoolFee(int(pool_fee*2**64), {"from": gov})
    sp2.setGovernanceFee(int(gov_fee*2**64), {"from": gov})


    # 1. Successful swap
    init_group_invariant = get_amp_swappool_group_invariant([[sp1, swappool1_info.tokens], [sp2, swappool2_info.tokens]])
    init_gov_from_asset_balance = swappool1_info.tokens[0].balanceOf(gov)

    swap_amount = 10**18
    run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,                       # Note that 'gov' will be providing the assets
        ibc_gov            = gov,
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )

    # Verify group invariant increased (because of fee)
    assert_relative_error(get_amp_swappool_group_invariant([[sp1, swappool1_info.tokens], [sp2, swappool2_info.tokens]]), init_group_invariant, 5e-5, 2)  # Note this checks that the relative error must be AT LEAST 5e-5. This value is arbitrary (and dependent on the pool fee) and has been determined experimentally. This check is intended to verify that the group invariant increase is not insignificant.

    # Verify gov received a shared of the fee
    assert_relative_error(swappool1_info.tokens[0].balanceOf(gov), init_gov_from_asset_balance - swap_amount + int(swap_amount*pool_fee*gov_fee), 0, 1e-5)  # Note that the gov account is funding the swap (inside run_amp_swap), hence its balance will decrease by 'swap_amount', and will then increase due to the gov fee.



    # 2. Check timeout
    assert swappool1_info.tokens[0].balanceOf(swapper1) == 0    # Required for later check

    init_group_invariant = get_amp_swappool_group_invariant([[sp1, swappool1_info.tokens], [sp2, swappool2_info.tokens]])
    init_gov_from_asset_balance = swappool1_info.tokens[0].balanceOf(gov)

    swap_amount = 10**18
    run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,                       # Note that 'gov' will be providing the assets
        ibc_gov            = gov,
        force_timeout      = True,
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )

    # Verify group invariant increased (because of fee)
    assert_relative_error(get_amp_swappool_group_invariant([[sp1, swappool1_info.tokens], [sp2, swappool2_info.tokens]]), init_group_invariant, 5e-5, 2)  # Note this checks that the relative error must be AT LEAST 5e-5. This value is arbitrary (and dependent on the pool fee) and has been determined experimentally. This check is intended to verify that the group invariant increase is not insignificant.

    # Verify gov received a shared of the fee
    assert_relative_error(swappool1_info.tokens[0].balanceOf(gov), init_gov_from_asset_balance - swap_amount + int(swap_amount*pool_fee*gov_fee), 0, 1e-5)  # Note that the gov account is funding the swap (inside run_amp_swap), hence its balance will decrease by 'swap_amount', and will then increase due to the gov fee.

    # Make sure the swapper receives LESS assets because of the fee
    assert_relative_error(swappool1_info.tokens[0].balanceOf(swapper1), swap_amount - int(swap_amount*pool_fee), -1e-5, 0)



def test_timed_out_swap(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    gov,
    swapper1,
    swapper2,
    fn_isolation
):

    # 1. Swap
    swap_amount = 1e18
    swap_result = run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov,
        force_timeout      = True
    )

    # Make sure the operation reverted
    finish_swap_result = swap_result.run_finish_swap_result
    assert finish_swap_result.tx_receive_swap is None
    assert finish_swap_result.output is None
    assert finish_swap_result.tx_escrow_timeout is not None

    # Check swapper balances
    assert swappool1_info.tokens[0].balanceOf(swapper1) == swap_amount      # Source swapper recovers the funds
    assert swappool2_info.tokens[0].balanceOf(swapper2) == 0


def test_swap_too_large(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    gov,
    swapper1,
    swapper2,
    fn_isolation
):

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    # Swap one of the assets of pool 1
    swap_amount = int(swappool1_info.init_token_balances[0]/2)
    swap_result_1 = run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov
    )

    assert sp1.getUnitCapacity() == sp1._maxUnitCapacity()
    assert sp2.getUnitCapacity() < sp2._maxUnitCapacity()

    # Swap the other asset of pool 1 (exceed security limit)
    swap_amount_2 = int(swappool1_info.init_token_balances[1]/1.5)
    swap_result_2 = run_amp_swap(
        chainId             = chainId,
        swap_amount         = swap_amount_2,
        source_token_index  = 1,
        target_token_index  = 0,
        from_swappool_info  = swappool1_info,
        to_swappool_info    = swappool2_info,
        from_swapper        = swapper1,
        to_swapper          = swapper2,
        approx_out          = False,
        approx_in           = False,
        ibcemulator         = ibcemulator,
        token_gov           = gov,
        ibc_gov             = gov,
        allow_target_revert = True  # Expect revert of the receiveSwap tx
    )

    # Make sure the operation reverted
    finish_swap_result = swap_result_2.run_finish_swap_result
    assert finish_swap_result.revert_exception is not None
    assert finish_swap_result.revert_exception.args[0].args[0]['message'] == \
        'VM Exception while processing transaction: revert Swap exceeds security limit. Please wait'

    # Check swapper balances
    assert swappool1_info.tokens[1].balanceOf(swapper1) == swap_amount_2                                # Source swapper recovers the funds of the second swap
    assert swappool2_info.tokens[0].balanceOf(swapper2) == swap_result_1.run_finish_swap_result.output  # Target swapper only has the funds of the first swap


# TODO test including governance fee missing

def test_direct_receive_swap_invocation(
    swappool1_info,
    hacker,
    fn_isolation
):
    sp = swappool1_info.swappool

    # Try to directly invoke receiveSwap
    with brownie.reverts(): # TODO dev msg
        sp.receiveSwap(
            0,
            hacker,
            2**64,
            0,
            False,
            brownie.convert.to_bytes(0x123456789abcdef123456789abcdef123456789abcdef123456789abcdef),
            {"from": hacker}
        )


def test_direct_escrow_ack_timeout_invocation(
    chainId,
    swappool1_info,
    swappool2_info,
    swapper1,
    swapper2,
    hacker,
    ibcemulator,
    gov,
    fn_isolation
):
    sp1 = swappool1_info.swappool

    # Create an escrow
    swap_amount = int(swappool1_info.init_token_balances[0]/2)
    swap_result = run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov,
        finish_swap        = False
    )

    message_hash      = swap_result.tx_send_swap.events['SendSwap'][0]['messageHash']
    transferred_units = swap_result.tx_send_swap.events['SendSwap']['output']
    fromAsset         = swap_result.tx_send_swap.events['SendSwap']["fromAsset"]
    escrowAmount      = decodePayload(swap_result.tx_send_swap.events["IncomingPacket"]["packet"][3])["_escrowAmount"]

    # Try to directly invoke ack
    with brownie.reverts(): # TODO dev msg
        sp1.sendSwapAck(message_hash, transferred_units, escrowAmount, fromAsset, {"from": hacker})

    # Try to directly invoke timeout
    with brownie.reverts(): # TODO dev msg
        sp1.sendSwapTimeout(message_hash, transferred_units, escrowAmount, fromAsset, {"from": hacker})




def test_swap_finish_with_manipulated_packet(
    chainId,
    swappool1_info,
    swappool2_info,
    swapper1,
    swapper2,
    ibcemulator,
    gov,
    fn_isolation
):
    sp1 = swappool1_info.swappool

    # Create an escrow
    swap_amount = 10**18
    swap_result = run_amp_swap(
        chainId            = chainId,
        swap_amount        = swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,
        ibc_gov            = gov,
        finish_swap        = False
    )

    # Try to finish the swap
    ibc_target_contract = swap_result.tx_send_swap.events["IncomingMetadata"]["metadata"][0]
    ibc_packet          = swap_result.tx_send_swap.events["IncomingPacket"]["packet"]

    # Manipulate 'units' from within the packet
    data = ibc_packet[3]
    increased_units = int.from_bytes(data[97:129], 'big') * 2
    malicious_data = \
        data[0:97]                          + \
        increased_units.to_bytes(32, 'big') + \
        data[129:]

    malicious_ibc_packet = (*ibc_packet[:3], malicious_data, *ibc_packet[4:])

    with brownie.reverts(): # TODO dev msg
        ibcemulator.ack(
            ibc_target_contract,
            malicious_ibc_packet,
            {"from": gov},
        )



def test_swap_from_asset_not_in_pool(
    chainId,
    swappool1_info,
    swappool2_info,
    swapper1,
    swapper2,
    gov,
    tokens,
    fn_isolation
):
    """
        Swap from an asset that is not in the source pool.
    """

    sp1 = swappool1_info.swappool
    sp2 = swappool2_info.swappool

    from_token = tokens[2]

    assert from_token not in swappool1_info.tokens

    swap_amount = 10**18

    # Give source token allowance to swapper
    from_token.transfer(swapper1, swap_amount, {"from": gov})
    from_token.approve(sp1, swap_amount, {"from": swapper1})

    # Send maliciously assets (the input token) to the pool to avoid getting division by 0 error
    from_token.transfer(sp1, 10**18, {"from": gov})

    # TODO: The transaction does not fail if the user sends some from_tokens to the pool before invoking the swap.
    # Add a require statement to make the error more explicit?
    with brownie.reverts():
        tx = sp1.sendSwap(
            chainId,
            brownie.convert.to_bytes(sp2.address.replace("0x", "")),
            brownie.convert.to_bytes(swapper2.address.replace("0x", "")),
            from_token,
            0,
            swap_amount,
            0,
            0,
            swapper1,
            {"from": swapper1}
        )

    # assert tx.events['SendSwap'][0]['output'] == 0



def test_swap_with_calldata(
    chainId,
    swappool1_info,
    swappool2_info,
    ibcemulator,
    swapper1,
    swapper2,
    gov,
    dummy_target_contract,
    fn_isolation
):
    """
        Explicitly test the calldata functionality
    """

    # 1. Swap
    initial_swap_amount = 10**18
    result_1 = run_amp_swap(
        chainId            = chainId,
        swap_amount        = initial_swap_amount,
        source_token_index = 0,
        target_token_index = 0,
        from_swappool_info = swappool1_info,
        to_swappool_info   = swappool2_info,
        from_swapper       = swapper1,
        to_swapper         = swapper2,
        approx_out         = False,
        approx_in          = False,
        ibcemulator        = ibcemulator,
        token_gov          = gov,                       # Note that 'gov' will be providing the assets
        ibc_gov            = gov,
        call_data          = (brownie.convert.to_bytes(dummy_target_contract.address, type_str="bytes32") + brownie.convert.to_bytes(0x1234, type_str="bytes2")),
        large_error_bound  = 0.01,   # TODO note larger error.
        small_error_bound  = 1e-4    # TODO note larger error.
    )
    swap_yield = result_1.run_finish_swap_result.output

    # Verify call data was executed
    call_data_event = result_1.run_finish_swap_result.tx_receive_swap.events['OnCatalystCallReceived']
    assert call_data_event['purchasedTokens'] == swap_yield
    assert call_data_event['data']            == "0x1234"
