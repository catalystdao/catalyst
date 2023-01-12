from dataclasses import dataclass
from math import log2
from typing import Any, Union

import brownie
from brownie import chain, ZERO_ADDRESS
from brownie.network.transaction import TransactionReceipt

from .utils import assert_relative_error, relative_error
from .deposit_utils import get_amp_swappool_group_invariant, get_swappool_group_invariant
from .swap_utils import get_swappool_spot_prices



@dataclass
class RunFinishLiquiditySwapResult:
    tx_swap_from_liquidity_units : Union[TransactionReceipt, None]
    tx_escrow_ack                : Union[TransactionReceipt, None]
    tx_escrow_timeout            : Union[TransactionReceipt, None]
    output                       : Union[int, None]       # output is None if the transaction fails/times out
    revert_exception             : Any

@dataclass
class RunLiquiditySwapResult:
    tx_swap_to_liquidity_units       : TransactionReceipt
    units                            : int
    run_finish_liquidity_swap_result : Union[RunFinishLiquiditySwapResult, None]


def run_liquidity_swap(
    chainId,
    swap_amount,
    from_swappool_info,
    to_swappool_info,
    from_swapper,
    to_swapper,
    approx_out,
    approx_in,
    ibcemulator,
    fallback_user       = None,
    finish_swap         = True,
    allow_target_revert = False,    # Only used for finish_swap = True
    force_timeout       = False,    # Only used for finish_swap = True
    ibc_gov             = None,     # Only used for finish_swap = True
    large_error_bound   = 1e-4,
    small_error_bound   = 1e-6
) -> RunLiquiditySwapResult:
    assert from_swappool_info.amplification == 2**64
    assert to_swappool_info.amplification   == 2**64

    sp1         = from_swappool_info.swappool
    sp1_tokens  = from_swappool_info.tokens
    sp1_weights = from_swappool_info.token_weights

    sp2         = to_swappool_info.swappool
    sp2_tokens  = to_swappool_info.tokens
    sp2_weights = to_swappool_info.token_weights

    fallback_user = from_swapper if fallback_user is None else fallback_user


    # Get state before swapping
    init_sp1_pool_token_supply        = sp1.totalSupply()
    init_sp1_escrowed_pool_tokens     = sp1._escrowedPoolTokens()
    init_sp1_from_swapper_pool_tokens = sp1.balanceOf(from_swapper)
    init_sp1_spot_prices              = get_swappool_spot_prices(sp1, sp1_tokens)

    init_group_invariant              = get_swappool_group_invariant([[sp1, sp1_tokens], [sp2, sp2_tokens]])

    # TODO security limit?
    # init_sp1_unit_capacity    = sp1.getUnitCapacity()

    # init_timestamp             = chain[-1].timestamp


    # 1. Perform Swap
    tx_swap_to_liquidity_units = sp1.outLiquidity(
        chainId,
        brownie.convert.to_bytes(sp2.address.replace("0x", "")),
        brownie.convert.to_bytes(to_swapper.address.replace("0x", "")),
        swap_amount,
        0,
        (approx_out and 1) | (approx_in and 2),
        fallback_user,
        {"from": from_swapper},
    )

    # Check transaction event
    swap_to_liquidity_units_events = tx_swap_to_liquidity_units.events['SwapToLiquidityUnits']
    assert len(swap_to_liquidity_units_events) == 1

    swap_to_liquidity_units_event = swap_to_liquidity_units_events[0]
    assert swap_to_liquidity_units_event['targetPool']   == sp2.address
    assert swap_to_liquidity_units_event['targetUser']   == to_swapper
    assert swap_to_liquidity_units_event['input']        == swap_amount
    #swap_to_liquidity_units_event['fees']            # TODO?

    output_liquidity_units = swap_to_liquidity_units_event['output']
    message_hash           = swap_to_liquidity_units_event['messageHash']

    # Check balances
    assert sp1.totalSupply()           == init_sp1_pool_token_supply        - swap_amount
    assert sp1.balanceOf(from_swapper) == init_sp1_from_swapper_pool_tokens - swap_amount

    # Check escrow 
    assert sp1._escrowedPoolTokens() == init_sp1_escrowed_pool_tokens + swap_amount
    escrow_info = sp1._escrowedLiquidityFor(message_hash)
    assert escrow_info == fallback_user
    
    # TODO Check group invariant?

    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp1_spot_prices, get_swappool_spot_prices(sp1, sp1_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound

    # Check transferred units
    expected_transferred_units_f = 0
    for i, token in enumerate(sp1_tokens):
        expected_transferred_units_f += sp1_weights[i] * log2(init_sp1_balance0[i]/expected_sp1_new_balance0[i])

    # ! Units must be equal or SMALLER than the expected one
    assert_relative_error(output_liquidity_units, int(expected_transferred_units_f * 2**64), -large_error_bound, small_error_bound)

    # TODO security limit
    # # Check security limit remains unchanged (note that it may have decayed since the tx was mined)
    # assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
    #     ref_capacity            = init_sp1_unit_capacity,
    #     ref_capacity_timestamp  = init_timestamp,
    #     change_capacity_delta   = 0,
    #     change_timestamp        = tx_swap_to_units.timestamp,
    #     current_timestamp       = chain[-1].timestamp,
    #     max_capacity            = sp1._max_unit_inflow()
    # )) <= 1 # Only allow for a very small rounding error


    # 2. Finish swap
    run_finish_liquidity_swap_result = None
    if finish_swap:

        if ibc_gov is None:
            raise RuntimeError('Can\'t finish swap without a provided ibc_gov')

        run_finish_liquidity_swap_result = run_finish_liquidity_swap(
            tx_swap_to_liquidity_units = tx_swap_to_liquidity_units,
            swap_amount                = swap_amount,
            from_swappool_info         = from_swappool_info,
            to_swappool_info           = to_swappool_info,
            fallback_user              = fallback_user,
            to_swapper                 = to_swapper,
            ibcemulator                = ibcemulator,
            ibc_gov                    = ibc_gov,
            allow_target_revert        = allow_target_revert,
            force_timeout              = force_timeout,
            large_error_bound          = large_error_bound,
            small_error_bound          = small_error_bound
        )
    
    return RunLiquiditySwapResult(
        tx_swap_to_liquidity_units        = tx_swap_to_liquidity_units,
        units                             = output_liquidity_units,
        run_finish_liquidity_swap_result  = run_finish_liquidity_swap_result
    )



def run_finish_liquidity_swap(
    tx_swap_to_liquidity_units,
    swap_amount,
    from_swappool_info,
    to_swappool_info,
    fallback_user,
    to_swapper,
    ibcemulator,
    ibc_gov,
    allow_target_revert = False,
    force_timeout       = False,
    large_error_bound   = 1e-4,
    small_error_bound   = 1e-6
) -> RunFinishLiquiditySwapResult:

    assert from_swappool_info.amplification == 2**64
    assert to_swappool_info.amplification   == 2**64

    sp1         = from_swappool_info.swappool
    sp1_tokens  = from_swappool_info.tokens

    sp2         = to_swappool_info.swappool
    sp2_tokens  = to_swappool_info.tokens
    sp2_weights = to_swappool_info.token_weights

    init_sp1_balance0                 = [sp1._balance0(token) for token in sp1_tokens]
    init_sp1_pool_token_supply        = sp1.totalSupply()
    init_sp1_escrowed_pool_tokens     = sp1._escrowedPoolTokens()
    init_sp1_spot_prices              = get_swappool_spot_prices(sp1, sp1_tokens)

    init_group_invariant              = get_swappool_group_invariant([[sp1, sp1_tokens], [sp2, sp2_tokens]])

    init_sp2_balance0               = [sp2._balance0(token) for token in sp2_tokens]
    init_sp2_pool_token_supply      = sp2.totalSupply()
    init_sp2_to_swapper_pool_tokens = sp2.balanceOf(to_swapper)
    init_sp2_spot_prices            = get_swappool_spot_prices(sp2, sp2_tokens)

    # TODO security limit
    # init_sp2_unit_capacity         = sp2.getUnitCapacity()

    # init_timestamp             = chain[-1].timestamp

    # Grab info from initial SwapToLiquidityUnits transaction
    ibc_target_contract = tx_swap_to_liquidity_units.events["IncomingMetadata"]["metadata"][0]
    ibc_packet          = tx_swap_to_liquidity_units.events["IncomingPacket"]["packet"]
    transferred_units   = tx_swap_to_liquidity_units.events['SwapToLiquidityUnits']['output']
    message_hash        = tx_swap_to_liquidity_units.events['SwapToLiquidityUnits']['messageHash']

    # 2a. Execute IBC package and ack swap if successful
    revert_exception = None
    if not force_timeout:

        try:
            tx_swap_from_liquidity_units = ibcemulator.execute(
                ibc_target_contract,
                ibc_packet,
                {"from": ibc_gov},
            )

        except brownie.exceptions.VirtualMachineError as e:
            # If SwapFromLiquidityUnits is unsuccesful, continue only if allowed by the test conditions
            if not allow_target_revert:
                raise e
            revert_exception = e
        
        else:
            # If SwapFromLiquidityUnits is successful, send ack

            # Check transaction event
            swap_from_liquidity_units_events = tx_swap_from_liquidity_units.events['SwapFromLiquidityUnits']
            assert len(swap_from_liquidity_units_events) == 1

            swap_from_liquidity_units_event = swap_from_liquidity_units_events[0]
            assert swap_from_liquidity_units_event['who']     == to_swapper
            assert swap_from_liquidity_units_event['input']   == transferred_units

            pool_tokens_yield = swap_from_liquidity_units_event['output']

            # TODO security limit
            # # Check security limit on the destination side (note that it may have decayed since the tx was mined)
            # assert abs(sp2.getUnitCapacity() - get_expected_decayed_units_capacity(
            #     ref_capacity            = init_sp2_unit_capacity,
            #     ref_capacity_timestamp  = init_timestamp,
            #     change_capacity_delta   = -transferred_units,
            #     change_timestamp        = tx_swap_from_units.timestamp,
            #     current_timestamp       = chain[-1].timestamp,
            #     max_capacity            = sp2._max_unit_inflow()
            # )) <= 1 # Only allow for a very small rounding error

            # Check balance0s
            distribution_factor = (2**((transferred_units/2**64)/sum(sp2_weights)) - 1)
            
            expected_sp2_new_balance0 = []
            for i, token in enumerate(sp2_tokens):
                expected_new_balance0 = init_sp2_balance0[i]*(1 + distribution_factor)
                # ! balance 0 must be equal or SMALLER than the expected one
                assert_relative_error(sp2._balance0(token), expected_new_balance0, -large_error_bound, small_error_bound)

                expected_sp2_new_balance0.append(expected_new_balance0)

            # Check output
            expected_received_pool_tokens = init_sp2_pool_token_supply * distribution_factor
            # ! Pool token supply must be equal or SMALLER than the expected one
            assert_relative_error(pool_tokens_yield, expected_received_pool_tokens, -large_error_bound, small_error_bound)

            # Check pool tokens
            assert sp2.totalSupply()         == init_sp2_pool_token_supply      + pool_tokens_yield
            assert sp2.balanceOf(to_swapper) == init_sp2_to_swapper_pool_tokens + pool_tokens_yield
            
            # # Check spot prices    # TODO
            # for init_spot_price, new_spot_price in zip(init_sp2_spot_prices, get_swappool_spot_prices(sp2, sp2_tokens)):
            #     assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound


            # # Check the group invariant # TODO
            # assert_relative_error(get_swappool_group_invariant([[sp1, sp1_tokens], [sp2, sp2_tokens]]), init_group_invariant, -small_error_bound, large_error_bound)


            init_sp1_fallback_user_pool_tokens = sp1.balanceOf(fallback_user)

            # Execute ack to release escrow
            tx_escrow_ack = ibcemulator.ack(
                ibc_target_contract,
                ibc_packet,
                {"from": ibc_gov}
            )

            # Escrow check
            assert sp1._escrowedPoolTokens() == init_sp1_escrowed_pool_tokens - swap_amount # Escrowed balance has decreased
    
            # Escrow info has been deleted
            assert sp1._escrowedLiquidityFor(message_hash) == ZERO_ADDRESS

            # Escrow event
            escrow_ack_events = tx_escrow_ack.events['EscrowAck']
            assert len(escrow_ack_events) == 1

            escrow_ack_event = escrow_ack_events[0]
            assert escrow_ack_event['messageHash']   == message_hash
            assert escrow_ack_event['liquiditySwap'] == True

            # Check balances
            assert sp1.balanceOf(fallback_user) == init_sp1_fallback_user_pool_tokens
            assert sp1.totalSupply()            == init_sp1_pool_token_supply

            # TODO check balance0?

            # TODO security limit
            # # Check security limit on the source side (note that it may have decayed since the tx was mined)
            # assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
            #     ref_capacity            = init_sp1_unit_capacity,
            #     ref_capacity_timestamp  = init_timestamp,
            #     change_capacity_delta   = transferred_units,
            #     change_timestamp        = tx_escrow_ack.timestamp,
            #     current_timestamp       = chain[-1].timestamp,
            #     max_capacity            = sp1._max_unit_inflow()
            # )) <= 1 # Only allow for a very small rounding error

            return RunFinishLiquiditySwapResult(
                tx_swap_from_liquidity_units = tx_swap_from_liquidity_units,
                tx_escrow_ack                = tx_escrow_ack,
                tx_escrow_timeout            = None,
                output                       = pool_tokens_yield,
                revert_exception             = revert_exception
            )


    # 2b. Execute timeout

    init_sp1_fallback_user_pool_tokens = sp1.balanceOf(fallback_user)

    # If the code reaches this point, either the timeout has been forced, or the target tx has been allowed to fail
    tx_escrow_timeout = ibcemulator.timeout(
        ibc_target_contract,
        ibc_packet,
        {"from": ibc_gov},
    )

    # Escrow check
    assert sp1._escrowedPoolTokens() == init_sp1_escrowed_pool_tokens - swap_amount # Escrowed balance has decreased
    assert sp1._escrowedLiquidityFor(message_hash) == ZERO_ADDRESS  # Escrow info has been deleted

    # Escrow event
    escrow_timeout_events = tx_escrow_timeout.events['EscrowTimeout']
    assert len(escrow_timeout_events) == 1

    escrow_timeout_event = escrow_timeout_events[0]
    assert escrow_timeout_event['messageHash']   == message_hash
    assert escrow_timeout_event['liquiditySwap'] == True

    # Check balances
    assert sp1.balanceOf(fallback_user) == init_sp1_fallback_user_pool_tokens + swap_amount
    assert sp1.totalSupply()            == init_sp1_pool_token_supply + swap_amount

    # TODO check balance0s
    # TODO check group invariant

    return RunFinishLiquiditySwapResult(
        tx_swap_from_liquidity_units  = None,
        tx_escrow_ack                 = None,
        tx_escrow_timeout             = tx_escrow_timeout,
        output                        = None,
        revert_exception              = revert_exception
    )



def run_amp_liquidity_swap(
    chainId,
    swap_amount,
    from_swappool_info,
    to_swappool_info,
    approx_out,
    approx_in,
    depositor,
    ibcemulator,
    gov,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
):

    sp1         = from_swappool_info.swappool
    sp1_tokens  = from_swappool_info.tokens
    sp1_weights = from_swappool_info.token_weights
    sp1_amp_f   = from_swappool_info.amplification / 2**64

    sp2         = to_swappool_info.swappool
    sp2_tokens  = to_swappool_info.tokens
    sp2_weights = to_swappool_info.token_weights
    sp2_amp_f   = to_swappool_info.amplification / 2**64


    # Get state before swapping
    init_sp1_pool_token_supply     = sp1.totalSupply()
    init_sp1_depositor_pool_tokens = sp1.balanceOf(depositor)
    init_sp1_balance0              = [sp1._balance0(token) for token in sp1_tokens]
    init_sp1_spot_prices           = get_swappool_spot_prices(sp1, sp1_tokens)

    init_group_invariant           = get_amp_swappool_group_invariant([[sp1, sp1_tokens], [sp2, sp2_tokens]])


    # 1. Perform swap
    tx = sp1.outLiquidity(
        chainId,
        brownie.convert.to_bytes(sp2.address.replace("0x", "")),
        brownie.convert.to_bytes(depositor.address.replace("0x", "")),
        swap_amount,
        0,
        (approx_out and 1) | (approx_in and 2),
        depositor,
        {"from": depositor},
    )

    # Check pool tokens
    assert sp1.totalSupply()        == init_sp1_pool_token_supply - swap_amount
    assert sp1.balanceOf(depositor) == init_sp1_depositor_pool_tokens - swap_amount

    # Check balance0s
    expected_sp1_new_balance0 = []
    for i, token in enumerate(sp1_tokens):
        expected_new_balance0 = init_sp1_balance0[i] * ( 1 - swap_amount/init_sp1_pool_token_supply )
        assert_relative_error(sp1._balance0(token), expected_new_balance0, -small_error_bound, large_error_bound)

        expected_sp1_new_balance0.append(expected_new_balance0)
    
    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp1_spot_prices, get_swappool_spot_prices(sp1, sp1_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound

    # Check transferred units
    expected_transferred_units_f = 0
    for i, token in enumerate(sp1_tokens):
        expected_transferred_units_f += sp1_weights[i] * (
            init_sp1_balance0[i]**(1 - sp1_amp_f) - (expected_sp1_new_balance0[i])**(1 - sp1_amp_f)
        )

    # ! Units must be equal or SMALLER than the expected one
    assert_relative_error(tx.events["SwapToLiquidityUnits"]["output"], expected_transferred_units_f * 2**64, -large_error_bound, small_error_bound)



    # 2. Execute the IBC package.
    # Get state before receiving swap (these are here and not at the beginning of the function in case sp1 == sp2, i.e. swapping with itself)
    init_sp2_pool_token_supply     = sp2.totalSupply()
    init_sp2_depositor_pool_tokens = sp2.balanceOf(depositor)
    init_sp2_balance0              = [sp2._balance0(token) for token in sp2_tokens]
    init_sp2_spot_prices           = get_swappool_spot_prices(sp2, sp2_tokens)

    ibcemulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": gov},
    )

    # Check balance0s
    weights_sum = sum([weight * init_sp2_balance0[i] ** (1 - sp2_amp_f) for i, weight in enumerate(sp2_weights)])

    distribution_factor = ((weights_sum + expected_transferred_units_f) / weights_sum) ** (1 / (1 - sp2_amp_f)) - 1

    expected_sp2_new_balance0 = []
    for i, token in enumerate(sp2_tokens):
        expected_new_balance0 = init_sp2_balance0[i]*(1 + distribution_factor)
        # ! balance 0 must be equal or SMALLER than the expected one
        assert_relative_error(sp2._balance0(token), expected_new_balance0, -large_error_bound, small_error_bound)

        expected_sp2_new_balance0.append(expected_new_balance0)

    expected_received_pool_tokens = init_sp2_pool_token_supply * distribution_factor

    # Check pool tokens
    # ! Pool token supply must be equal or SMALLER than the expected one
    assert_relative_error(sp2.totalSupply(), init_sp2_pool_token_supply + expected_received_pool_tokens, -large_error_bound, small_error_bound)
    assert_relative_error(sp2.balanceOf(depositor), init_sp2_depositor_pool_tokens + expected_received_pool_tokens, -large_error_bound, small_error_bound)
    
    # Check spot prices
    for init_spot_price, new_spot_price in zip(init_sp2_spot_prices, get_swappool_spot_prices(sp2, sp2_tokens)):
        assert abs(relative_error(init_spot_price, new_spot_price)) < large_error_bound


    # Check the group invariant
    assert_relative_error(get_amp_swappool_group_invariant([[sp1, sp1_tokens], [sp2, sp2_tokens]]), init_group_invariant, -small_error_bound, large_error_bound)
    
    # Execute ack to release escrow # TODO: Implement timeout.
    ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": gov}
    ) 
