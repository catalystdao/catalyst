from dataclasses import dataclass
from math import log2
from typing import Any, Union

import brownie
from brownie import chain, ZERO_ADDRESS, convert
from brownie.network.transaction import TransactionReceipt

from .utils import assert_relative_error, get_expected_decayed_units_capacity


@dataclass
class RunLocalSwapResult:
    tx     : Any
    output : int

@dataclass
class RunFinishSwapResult:
    tx_swap_from_units : Union[TransactionReceipt, None]
    tx_escrow_ack      : Union[TransactionReceipt, None]
    tx_escrow_timeout  : Union[TransactionReceipt, None]
    output             : Union[int, None]       # output is None if the transaction fails/times out
    revert_exception   : Any

@dataclass
class RunSwapResult:
    tx_swap_to_units       : TransactionReceipt
    units                  : int
    run_finish_swap_result : Union[RunFinishSwapResult, None]

def run_local_swap(
    swap_amount,
    source_token_index,
    target_token_index,
    swappool_info,
    swapper,
    approx,
    gov,
    min_amount = 0,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
) -> RunLocalSwapResult:

    assert swappool_info.amplification == 2**64

    sp         = swappool_info.swappool
    sp_tokens  = swappool_info.tokens
    sp_weights = swappool_info.token_weights

    from_token                = sp_tokens[source_token_index]
    from_token_weight         = sp_weights[source_token_index]
    init_from_swapper_balance = from_token.balanceOf(swapper)
    init_from_sp_balance      = from_token.balanceOf(sp)

    to_token                  = sp_tokens[target_token_index]
    to_token_weight           = sp_weights[target_token_index]
    init_to_swapper_balance   = to_token.balanceOf(swapper)
    init_to_sp_balance        = to_token.balanceOf(sp)

    # Make sure swapper has funds
    if gov is not None and swap_amount - init_from_swapper_balance > 0:
        from_token.transfer(swapper, swap_amount - init_from_swapper_balance, {"from": gov})
        init_from_swapper_balance = swap_amount
    
    # Approve funds for the swap pool
    from_token.approve(sp, swap_amount, {"from": swapper})

    
    # Perform Swap
    if approx is None:  # Allow testing of both methods signatures (for coverage purposes)
        tx = sp.localswap(
            from_token,
            to_token,
            swap_amount,
            min_amount,
            {"from": swapper}
        )
    else:
        tx = sp.localswap(
            from_token,
            to_token,
            swap_amount,
            min_amount,
            approx,
            {"from": swapper}
        )
    
    # Check transaction event
    local_swap_events = tx.events['LocalSwap']
    assert len(local_swap_events) == 1

    swap_event = local_swap_events[0]
    assert swap_event['who']       == swapper
    assert swap_event['fromAsset'] == from_token
    assert swap_event['toAsset']   == to_token
    assert swap_event['input']     == swap_amount

    # Check output
    observed_output = swap_event['output']
    expected_output = int(init_to_sp_balance * (1 - (init_from_sp_balance/(init_from_sp_balance + swap_amount))**(from_token_weight/to_token_weight) ))
    assert_relative_error(observed_output, expected_output, -large_error_bound, small_error_bound, error_id="LOCAL_SWAP_RETURN_ERROR")
 
    assert observed_output >= min_amount

    # Check balances
    assert from_token.balanceOf(swapper) == init_from_swapper_balance - swap_amount
    assert from_token.balanceOf(sp)      == init_from_sp_balance + swap_amount

    assert to_token.balanceOf(swapper)   == init_to_swapper_balance + observed_output
    assert to_token.balanceOf(sp)        == init_to_sp_balance - observed_output


    return RunLocalSwapResult(tx, observed_output)



def run_amp_local_swap(
    swap_amount,
    source_token_index,
    target_token_index,
    swappool_info,
    swapper,
    approx,
    gov,
    min_amount = 0,
    large_error_bound = 1e-4,
    small_error_bound = 1e-6
) -> RunLocalSwapResult:

    assert swappool_info.amplification != 2**64

    sp         = swappool_info.swappool
    sp_tokens  = swappool_info.tokens
    sp_weights = swappool_info.token_weights
    sp_amp_f   = swappool_info.amplification / 2**64

    from_token                = sp_tokens[source_token_index]
    from_token_weight         = sp_weights[source_token_index]
    init_from_swapper_balance = from_token.balanceOf(swapper)
    init_from_sp_balance      = from_token.balanceOf(sp)

    to_token                  = sp_tokens[target_token_index]
    to_token_weight           = sp_weights[target_token_index]
    init_to_swapper_balance   = to_token.balanceOf(swapper)
    init_to_sp_balance        = to_token.balanceOf(sp)

    # Make sure swapper has funds
    if gov is not None and swap_amount - init_from_swapper_balance > 0:
        from_token.transfer(swapper, swap_amount - init_from_swapper_balance, {"from": gov})
        init_from_swapper_balance = swap_amount
    
    # Approve funds for the swap pool
    from_token.approve(sp, swap_amount, {"from": swapper})

    
    # Perform Swap
    if approx is None:  # Allow testing of both methods signatures (for coverage purposes)
        tx = sp.localswap(
            from_token,
            to_token,
            swap_amount,
            min_amount,
            {"from": swapper}
        )
    else:
        tx = sp.localswap(
            from_token,
            to_token,
            swap_amount,
            min_amount,
            approx,
            {"from": swapper}
        )
    
    # Check transaction event
    local_swap_events = tx.events['LocalSwap']
    assert len(local_swap_events) == 1

    swap_event = local_swap_events[0]
    assert swap_event['who']       == swapper
    assert swap_event['fromAsset'] == from_token
    assert swap_event['toAsset']   == to_token
    assert swap_event['input']     == swap_amount

    # Check output
    observed_output = swap_event['output']
    one_minus_amp   = 1 - sp_amp_f
    expected_output = int(
        init_to_sp_balance * (
            1 - (
                1 - from_token_weight * (
                    (init_from_sp_balance + swap_amount)**one_minus_amp - init_from_sp_balance**one_minus_amp
                ) / (to_token_weight * init_to_sp_balance**one_minus_amp)
            )**(1/one_minus_amp)
        )
    )
    assert_relative_error(observed_output, expected_output, -large_error_bound, small_error_bound, error_id="LOCAL_SWAP_RETURN_ERROR")
 
    assert observed_output >= min_amount

    # Check balances
    assert from_token.balanceOf(swapper) == init_from_swapper_balance - swap_amount
    assert from_token.balanceOf(sp)      == init_from_sp_balance + swap_amount

    assert to_token.balanceOf(swapper)   == init_to_swapper_balance + observed_output
    assert to_token.balanceOf(sp)        == init_to_sp_balance - observed_output


    return RunLocalSwapResult(tx, observed_output)



def run_swap(
    chainId,
    swap_amount,
    source_token_index,
    target_token_index,
    from_swappool_info,
    to_swappool_info,
    from_swapper,
    to_swapper,
    approx_out,
    approx_in,
    ibcemulator,
    call_data           = None,
    token_gov           = None,
    fallback_user       = None,
    min_amount          = 0,
    finish_swap         = True,
    allow_target_revert = False,    # Only used for finish_swap = True
    force_timeout       = False,    # Only used for finish_swap = True
    ibc_gov             = None,     # Only used for finish_swap = True
    large_error_bound   = 1e-4,
    small_error_bound   = 1e-6
) -> RunSwapResult:
    assert from_swappool_info.amplification == 2**64
    assert to_swappool_info.amplification   == 2**64

    sp1          = from_swappool_info.swappool
    sp1_tokens   = from_swappool_info.tokens
    sp1_weights  = from_swappool_info.token_weights
    sp1_pool_fee = from_swappool_info.pool_fee
    sp1_gov_fee  = from_swappool_info.governance_fee

    sp2          = to_swappool_info.swappool

    fallback_user = from_swapper if fallback_user is None else fallback_user

    from_token                = sp1_tokens[source_token_index]
    from_token_weight         = sp1_weights[source_token_index]

    init_from_swapper_balance = from_token.balanceOf(from_swapper)
    init_sp1_balance          = from_token.balanceOf(sp1)
    init_sp1_escrowed_balance = sp1._escrowedTokens(from_token)
    init_sp1_unit_capacity    = sp1.getUnitCapacity()

    init_timestamp            = chain[-1].timestamp

    # Make sure swapper has funds
    if token_gov is not None and swap_amount - init_from_swapper_balance > 0:
        from_token.transfer(from_swapper, swap_amount - init_from_swapper_balance, {"from": token_gov})
        init_from_swapper_balance = swap_amount
    
    # Approve funds for the swap pool
    from_token.approve(sp1, swap_amount, {"from": from_swapper})



    # 1. Perform Swap
    swap_to_units_args = [
        chainId,
        brownie.convert.to_bytes(sp2.address.replace("0x", "")),
        brownie.convert.to_bytes(to_swapper.address.replace("0x", "")),
        from_token,
        target_token_index,
        swap_amount,
        min_amount,
        (approx_out and 1) | (approx_in and 2),
        fallback_user
    ]
    if call_data is not None:
        swap_to_units_args.append(call_data)

    tx_swap_to_units = sp1.swapToUnits(*swap_to_units_args, {"from": from_swapper})

    # Expected fees
    expected_pool_fee = int(sp1_pool_fee * swap_amount)
    expected_gov_fee  = int(sp1_gov_fee * expected_pool_fee)

    # Check transaction event
    swap_to_units_events = tx_swap_to_units.events['SwapToUnits']
    assert len(swap_to_units_events) == 1

    swap_to_units_event = swap_to_units_events[0]
    assert swap_to_units_event['targetPool']   == sp2.address
    assert swap_to_units_event['targetUser']   == to_swapper
    assert swap_to_units_event['fromAsset']    == from_token
    assert swap_to_units_event['toAssetIndex'] == target_token_index
    assert swap_to_units_event['input']        == swap_amount

    output_units = swap_to_units_event['output']
    messageHash  = swap_to_units_event['messageHash']

    # Check balances
    assert from_token.balanceOf(from_swapper) == init_from_swapper_balance - swap_amount
    assert_relative_error(from_token.balanceOf(sp1), init_sp1_balance + swap_amount - expected_gov_fee, -small_error_bound, large_error_bound)

    # Check escrow
    escrow_info = sp1._escrowedFor(messageHash)
    assert escrow_info == fallback_user

    escrowed_amount = decodePayload(tx_swap_to_units.events["IncomingPacket"]["packet"][3])["_escrowAmount"]
    assert_relative_error(escrowed_amount, swap_amount - expected_pool_fee, -large_error_bound, small_error_bound)

    assert sp1._escrowedTokens(from_token) == init_sp1_escrowed_balance + escrowed_amount

    # Check transferred units
    expected_transferred_units_f = from_token_weight * log2((init_sp1_balance + escrowed_amount) / init_sp1_balance)
    
    # ! Units must be equal or SMALLER than the expected one
    assert_relative_error(output_units, expected_transferred_units_f * 2**64, -large_error_bound, small_error_bound)

    # Check security limit remains unchanged (note that it may have decayed since the tx was mined)
    assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
        ref_capacity            = init_sp1_unit_capacity,
        ref_capacity_timestamp  = init_timestamp,
        change_capacity_delta   = 0,
        change_timestamp        = tx_swap_to_units.timestamp,
        current_timestamp       = chain[-1].timestamp,
        max_capacity            = sp1._max_unit_inflow()
    )) <= 1 # Only allow for a very small rounding error


    # 2. Finish swap
    run_finish_swap_result = None
    if finish_swap:

        if ibc_gov is None:
            raise RuntimeError('Can\'t finish swap without a provided ibc_gov')

        run_finish_swap_result = run_finish_swap(
            tx_swap_to_units    = tx_swap_to_units,
            swap_amount         = escrowed_amount,
            source_token_index  = source_token_index,
            target_token_index  = target_token_index,
            from_swappool_info  = from_swappool_info,
            to_swappool_info    = to_swappool_info,
            fallback_user       = fallback_user,
            to_swapper          = to_swapper,
            ibcemulator         = ibcemulator,
            ibc_gov             = ibc_gov,
            min_amount          = min_amount,
            allow_target_revert = allow_target_revert,
            force_timeout       = force_timeout,
            large_error_bound   = large_error_bound,
            small_error_bound   = small_error_bound
        )
    
    return RunSwapResult(
        tx_swap_to_units        = tx_swap_to_units,
        units                   = output_units,
        run_finish_swap_result  = run_finish_swap_result
    )



def run_finish_swap(
    tx_swap_to_units,
    swap_amount,
    source_token_index,
    target_token_index,
    from_swappool_info,
    to_swappool_info,
    fallback_user,
    to_swapper,
    ibcemulator,
    ibc_gov,
    min_amount          = 0,
    allow_target_revert = False,
    force_timeout       = False,
    large_error_bound   = 1e-4,
    small_error_bound   = 1e-6
) -> RunFinishSwapResult:

    assert from_swappool_info.amplification == 2**64
    assert to_swappool_info.amplification   == 2**64

    successful_delivery = False


    # Grab info from initial swapToUnits transaction
    ibc_target_contract = tx_swap_to_units.events["IncomingMetadata"]["metadata"][0]
    ibc_packet          = tx_swap_to_units.events["IncomingPacket"]["packet"]
    transferred_units   = tx_swap_to_units.events['SwapToUnits']['output']
    messageHash         = tx_swap_to_units.events['SwapToUnits']['messageHash']



    # 1. Execute the IBC package on the target chain

    sp2                     = to_swappool_info.swappool
    sp2_tokens              = to_swappool_info.tokens
    sp2_weights             = to_swappool_info.token_weights

    to_token                = sp2_tokens[target_token_index]
    to_token_weight         = sp2_weights[target_token_index]

    init_to_swapper_balance = to_token.balanceOf(to_swapper)
    init_sp2_balance        = to_token.balanceOf(sp2)

    init_sp2_unit_capacity  = sp2.getUnitCapacity()
    init_sp2_timestamp      = chain[-1].timestamp

    tx_swap_from_units = None
    revert_exception   = None
    to_asset_yield     = None

    if not force_timeout:

        try:
            tx_swap_from_units = ibcemulator.execute(
                ibc_target_contract,
                ibc_packet,
                {"from": ibc_gov},
            )

        except brownie.exceptions.VirtualMachineError as e:
            # If SwapFromUnits is unsuccesful, continue only if allowed by the test conditions
            if not allow_target_revert:
                raise e
            revert_exception = e
        
        else:

            # Check transaction event
            swap_from_units_events = tx_swap_from_units.events['SwapFromUnits']
            assert len(swap_from_units_events) == 1

            swap_from_units_event = swap_from_units_events[0]
            assert swap_from_units_event['who']     == to_swapper
            assert swap_from_units_event['toAsset'] == to_token
            assert swap_from_units_event['input']   == transferred_units

            to_asset_yield = swap_from_units_event['output']

            # Check security limit on the destination side (note that it may have decayed since the tx was mined)
            assert abs(sp2.getUnitCapacity() - get_expected_decayed_units_capacity(
                ref_capacity            = init_sp2_unit_capacity,
                ref_capacity_timestamp  = init_sp2_timestamp,
                change_capacity_delta   = -transferred_units,
                change_timestamp        = tx_swap_from_units.timestamp,
                current_timestamp       = chain[-1].timestamp,
                max_capacity            = sp2._max_unit_inflow()
            )) <= 1 # Only allow for a very small rounding error

            # Check output
            expected_to_asset_yield = init_sp2_balance * (1 - 2**(-(transferred_units/2**64)/to_token_weight))
            assert_relative_error(to_asset_yield, expected_to_asset_yield, -large_error_bound, small_error_bound)

            assert to_asset_yield >= min_amount

            # Check balances
            assert to_token.balanceOf(to_swapper) == init_to_swapper_balance + to_asset_yield
            assert to_token.balanceOf(sp2)        == init_sp2_balance        - to_asset_yield

            # Note the transaction was successful
            successful_delivery = True
    


    # 2. Execute the IBC acknowledgement/timeout callback

    sp1                        = from_swappool_info.swappool
    sp1_tokens                 = from_swappool_info.tokens

    from_token                 = sp1_tokens[source_token_index]

    init_fallback_user_balance = from_token.balanceOf(fallback_user)
    init_sp1_balance           = from_token.balanceOf(sp1)

    init_sp1_escrowed_balance  = sp1._escrowedTokens(from_token)

    init_sp1_unit_capacity     = sp1.getUnitCapacity()
    init_sp1_timestamp         = chain[-1].timestamp

    # 2a Trigger ack if target was successful
    if successful_delivery:

        # Execute ack to release escrow
        tx_escrow_ack = ibcemulator.ack(
            ibc_target_contract,
            ibc_packet,
            {"from": ibc_gov}
        )

        # Escrow check
        assert sp1._escrowedTokens(from_token) == init_sp1_escrowed_balance - swap_amount # Escrowed balance has decreased

        # Escrow info has been deleted
        assert sp1._escrowedFor(messageHash) == ZERO_ADDRESS

        # Escrow event
        escrow_ack_events = tx_escrow_ack.events['EscrowAck']
        assert len(escrow_ack_events) == 1

        escrow_ack_event = escrow_ack_events[0]
        assert escrow_ack_event['messageHash']   == messageHash
        assert escrow_ack_event['liquiditySwap'] == False

        # Check balances
        assert from_token.balanceOf(fallback_user) == init_fallback_user_balance
        assert from_token.balanceOf(sp1)           == init_sp1_balance

        # Check security limit on the source side (note that it may have decayed since the tx was mined)
        assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
            ref_capacity            = init_sp1_unit_capacity,
            ref_capacity_timestamp  = init_sp1_timestamp,
            change_capacity_delta   = transferred_units,
            change_timestamp        = tx_escrow_ack.timestamp,
            current_timestamp       = chain[-1].timestamp,
            max_capacity            = sp1._max_unit_inflow()
        )) <= 1 # Only allow for a very small rounding error

        return RunFinishSwapResult(
            tx_swap_from_units  = tx_swap_from_units,
            tx_escrow_ack       = tx_escrow_ack,
            tx_escrow_timeout   = None,
            output              = to_asset_yield,
            revert_exception    = revert_exception
        )


    # 2b. Trigger timeout if target was unsuccessful/was requested by the test
    if not successful_delivery:

        # If the code reaches this point, either the timeout has been forced, or the target tx has been allowed to fail
        tx_escrow_timeout = ibcemulator.timeout(
            ibc_target_contract,
            ibc_packet,
            {"from": ibc_gov},
        )

        # Check security limit remains unchanged (note that it may have decayed since the tx was mined)
        assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
            ref_capacity            = init_sp1_unit_capacity,
            ref_capacity_timestamp  = init_sp1_timestamp,
            change_capacity_delta   = 0,
            change_timestamp        = tx_escrow_timeout.timestamp,
            current_timestamp       = chain[-1].timestamp,
            max_capacity            = sp1._max_unit_inflow()
        )) <= 1 # Only allow for a very small rounding error

        # Escrow check
        assert sp1._escrowedTokens(from_token) == init_sp1_escrowed_balance - swap_amount # Escrowed balance has decreased
        assert sp1._escrowedFor(messageHash) == ZERO_ADDRESS

        # Escrow event
        escrow_timeout_events = tx_escrow_timeout.events['EscrowTimeout']
        assert len(escrow_timeout_events) == 1

        escrow_timeout_event = escrow_timeout_events[0]
        assert escrow_timeout_event['messageHash']   == messageHash
        assert escrow_timeout_event['liquiditySwap'] == False

        # Check balances
        assert from_token.balanceOf(fallback_user) == init_fallback_user_balance + swap_amount
        assert from_token.balanceOf(sp1)           == init_sp1_balance - swap_amount

        return RunFinishSwapResult(
            tx_swap_from_units  = None,
            tx_escrow_ack       = None,
            tx_escrow_timeout   = tx_escrow_timeout,
            output              = None,
            revert_exception    = revert_exception
        )




def run_amp_swap(
    chainId,
    swap_amount,
    source_token_index,
    target_token_index,
    from_swappool_info,
    to_swappool_info,
    from_swapper,
    to_swapper,
    approx_out,
    approx_in,
    ibcemulator,
    call_data           = None,
    token_gov           = None,
    fallback_user       = None,
    min_amount          = 0,
    finish_swap         = True,
    allow_target_revert = False,    # Only used for finish_swap = True
    force_timeout       = False,    # Only used for finish_swap = True
    ibc_gov             = None,     # Only used for finish_swap = True
    large_error_bound   = 1e-4,
    small_error_bound   = 1e-6
) -> RunSwapResult:
    assert from_swappool_info.amplification != 2**64
    assert to_swappool_info.amplification   != 2**64

    sp1          = from_swappool_info.swappool
    sp1_tokens   = from_swappool_info.tokens
    sp1_weights  = from_swappool_info.token_weights
    sp1_amp_f    = from_swappool_info.amplification / 2**64
    sp1_pool_fee = from_swappool_info.pool_fee
    sp1_gov_fee  = from_swappool_info.governance_fee

    sp2          = to_swappool_info.swappool

    fallback_user = from_swapper if fallback_user is None else fallback_user

    from_token                = sp1_tokens[source_token_index]
    from_token_weight         = sp1_weights[source_token_index]

    init_from_swapper_balance = from_token.balanceOf(from_swapper)
    init_sp1_balance          = from_token.balanceOf(sp1)
    init_sp1_escrowed_balance = sp1._escrowedTokens(from_token)
    init_sp1_unit_capacity    = sp1.getUnitCapacity()

    init_timestamp             = chain[-1].timestamp

    # Make sure swapper has funds
    if token_gov is not None and swap_amount - init_from_swapper_balance > 0:
        from_token.transfer(from_swapper, swap_amount - init_from_swapper_balance, {"from": token_gov})
        init_from_swapper_balance = swap_amount
    
    # Approve funds for the swap pool
    from_token.approve(sp1, swap_amount, {"from": from_swapper})



    # 1. Perform Swap
    swap_to_units_args = [
        chainId,
        brownie.convert.to_bytes(sp2.address.replace("0x", "")),
        brownie.convert.to_bytes(to_swapper.address.replace("0x", "")),
        from_token,
        target_token_index,
        swap_amount,
        min_amount,
        (approx_out and 1) | (approx_in and 2),
        fallback_user
    ]
    if call_data is not None:
        swap_to_units_args.append(call_data)

    tx_swap_to_units = sp1.swapToUnits(*swap_to_units_args, {"from": from_swapper})

    # Expected fees
    expected_pool_fee = int(sp1_pool_fee * swap_amount)
    expected_gov_fee  = int(sp1_gov_fee * expected_pool_fee)

    # Check transaction event
    swap_to_units_events = tx_swap_to_units.events['SwapToUnits']
    assert len(swap_to_units_events) == 1

    swap_to_units_event = swap_to_units_events[0]
    assert swap_to_units_event['targetPool']   == sp2.address
    assert swap_to_units_event['targetUser']   == to_swapper
    assert swap_to_units_event['fromAsset']    == from_token
    assert swap_to_units_event['toAssetIndex'] == target_token_index
    assert swap_to_units_event['input']        == swap_amount

    output_units = swap_to_units_event['output']
    messageHash  = swap_to_units_event['messageHash']

    # Check balances
    assert from_token.balanceOf(from_swapper) == init_from_swapper_balance - swap_amount
    assert_relative_error(from_token.balanceOf(sp1), init_sp1_balance + swap_amount - expected_gov_fee, -small_error_bound, large_error_bound)

    # Check escrow 
    escrow_info = sp1._escrowedFor(messageHash)
    assert escrow_info == fallback_user

    escrowed_amount = decodePayload(tx_swap_to_units.events["IncomingPacket"]["packet"][3])["_escrowAmount"]
    assert_relative_error(escrowed_amount, swap_amount - expected_pool_fee, -large_error_bound, small_error_bound)

    assert sp1._escrowedTokens(from_token) == init_sp1_escrowed_balance + escrowed_amount

    # Check transferred units
    expected_transferred_units_f = from_token_weight * ((init_sp1_balance + escrowed_amount)**(1 - sp1_amp_f) - init_sp1_balance**(1 - sp1_amp_f))

    # ! Units must be equal or SMALLER than the expected one
    assert_relative_error(output_units, expected_transferred_units_f * 2**64, -large_error_bound, small_error_bound)

    # TODO enable once code is fixed
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
    run_finish_swap_result = None
    if finish_swap:

        if ibc_gov is None:
            raise RuntimeError('Can\'t finish swap without a provided ibc_gov')

        run_finish_swap_result = run_amp_finish_swap(
            tx_swap_to_units    = tx_swap_to_units,
            swap_amount         = escrowed_amount,
            source_token_index  = source_token_index,
            target_token_index  = target_token_index,
            from_swappool_info  = from_swappool_info,
            to_swappool_info    = to_swappool_info,
            fallback_user       = fallback_user,
            to_swapper          = to_swapper,
            ibcemulator         = ibcemulator,
            ibc_gov             = ibc_gov,
            min_amount          = min_amount,
            allow_target_revert = allow_target_revert,
            force_timeout       = force_timeout,
            large_error_bound   = large_error_bound,
            small_error_bound   = small_error_bound
        )
    
    return RunSwapResult(
        tx_swap_to_units        = tx_swap_to_units,
        units                   = output_units,
        run_finish_swap_result  = run_finish_swap_result
    )



def run_amp_finish_swap(
    tx_swap_to_units,
    swap_amount,
    source_token_index,
    target_token_index,
    from_swappool_info,
    to_swappool_info,
    fallback_user,
    to_swapper,
    ibcemulator,
    ibc_gov,
    min_amount          = 0,
    allow_target_revert = False,
    force_timeout       = False,
    large_error_bound   = 1e-4,
    small_error_bound   = 1e-6
) -> RunFinishSwapResult:

    assert from_swappool_info.amplification != 2**64
    assert to_swappool_info.amplification   != 2**64

    successful_delivery = False


    # Grab info from initial swapToUnits transaction
    ibc_target_contract = tx_swap_to_units.events["IncomingMetadata"]["metadata"][0]
    ibc_packet          = tx_swap_to_units.events["IncomingPacket"]["packet"]
    transferred_units   = tx_swap_to_units.events['SwapToUnits']['output']
    messageHash         = tx_swap_to_units.events['SwapToUnits']['messageHash']



    # 1. Execute the IBC package on the target chain
    sp2         = to_swappool_info.swappool
    sp2_tokens  = to_swappool_info.tokens
    sp2_weights = to_swappool_info.token_weights
    sp2_amp_f   = to_swappool_info.amplification / 2**64

    to_token                   = sp2_tokens[target_token_index]
    to_token_weight            = sp2_weights[target_token_index]

    init_to_swapper_balance    = to_token.balanceOf(to_swapper)
    init_sp2_balance           = to_token.balanceOf(sp2)

    init_sp2_unit_capacity     = sp2.getUnitCapacity()
    init_sp2_timestamp         = chain[-1].timestamp

    tx_swap_from_units = None
    revert_exception   = None
    to_asset_yield     = None

    if not force_timeout:

        try:
            tx_swap_from_units = ibcemulator.execute(
                ibc_target_contract,
                ibc_packet,
                {"from": ibc_gov},
            )

        except brownie.exceptions.VirtualMachineError as e:
            # If SwapFromUnits is unsuccesful, continue only if allowed by the test conditions
            if not allow_target_revert:
                raise e
            revert_exception = e
        
        else:
            # If SwapFromUnits is successful, send ack

            # Check transaction event
            swap_from_units_events = tx_swap_from_units.events['SwapFromUnits']
            assert len(swap_from_units_events) == 1

            swap_from_units_event = swap_from_units_events[0]
            assert swap_from_units_event['who']     == to_swapper
            assert swap_from_units_event['toAsset'] == to_token
            assert swap_from_units_event['input']   == transferred_units

            to_asset_yield = swap_from_units_event['output']

            # TODO security limit check
            # # Check security limit on the destination side (note that it may have decayed since the tx was mined)
            # assert abs(sp2.getUnitCapacity() - get_expected_decayed_units_capacity(
            #     ref_capacity            = init_sp2_unit_capacity,
            #     ref_capacity_timestamp  = init_timestamp,
            #     change_capacity_delta   = -transferred_units,
            #     change_timestamp        = tx_swap_from_units.timestamp,
            #     current_timestamp       = chain[-1].timestamp,
            #     max_capacity            = sp2._max_unit_inflow()
            # )) <= 1 # Only allow for a very small rounding error

            # Check output
            expected_to_asset_yield = init_sp2_balance * (1 - ( 1 - (transferred_units/2**64) / (to_token_weight * init_sp2_balance**(1 - sp2_amp_f)) )**(1/(1 - sp2_amp_f)) )
            assert_relative_error(to_asset_yield, expected_to_asset_yield, -large_error_bound, small_error_bound)

            assert to_asset_yield >= min_amount

            # Check balances
            assert to_token.balanceOf(to_swapper) == init_to_swapper_balance + to_asset_yield
            assert to_token.balanceOf(sp2)        == init_sp2_balance        - to_asset_yield
            
            # Note the transaction was successful
            successful_delivery = True
    


    # 2. Execute the IBC acknowledgement/timeout callback

    sp1         = from_swappool_info.swappool
    sp1_tokens  = from_swappool_info.tokens

    from_token                 = sp1_tokens[source_token_index]

    init_fallback_user_balance = from_token.balanceOf(fallback_user)
    init_sp1_balance           = from_token.balanceOf(sp1)

    init_sp1_escrowed_balance  = sp1._escrowedTokens(from_token)

    init_sp1_unit_capacity     = sp1.getUnitCapacity()
    init_sp1_timestamp         = chain[-1].timestamp

    # 2a. Execute IBC package and ack swap if successful
    if successful_delivery:

        # Execute ack to release escrow
        tx_escrow_ack = ibcemulator.ack(
            ibc_target_contract,
            ibc_packet,
            {"from": ibc_gov}
        )

        # Escrow check
        assert sp1._escrowedTokens(from_token) == init_sp1_escrowed_balance - swap_amount # Escrowed balance has decreased

        # Escrow info has been deleted
        assert sp1._escrowedFor(messageHash) == ZERO_ADDRESS

        # Escrow event
        escrow_ack_events = tx_escrow_ack.events['EscrowAck']
        assert len(escrow_ack_events) == 1

        escrow_ack_event = escrow_ack_events[0]
        assert escrow_ack_event['messageHash']   == messageHash
        assert escrow_ack_event['liquiditySwap'] == False

        # Check balances
        assert from_token.balanceOf(fallback_user) == init_fallback_user_balance
        assert from_token.balanceOf(sp1)           == init_sp1_balance

        # TODO enable after code is fixed
        # # Check security limit on the source side (note that it may have decayed since the tx was mined)
        # assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
        #     ref_capacity            = init_sp1_unit_capacity,
        #     ref_capacity_timestamp  = init_timestamp,
        #     change_capacity_delta   = transferred_units,
        #     change_timestamp        = tx_escrow_ack.timestamp,
        #     current_timestamp       = chain[-1].timestamp,
        #     max_capacity            = sp1._max_unit_inflow()
        # )) <= 1 # Only allow for a very small rounding error

        return RunFinishSwapResult(
            tx_swap_from_units  = tx_swap_from_units,
            tx_escrow_ack       = tx_escrow_ack,
            tx_escrow_timeout   = None,
            output              = to_asset_yield,
            revert_exception    = revert_exception
        )


    # 2b. Trigger timeout if target was unsuccessful/was requested by the test
    if not successful_delivery:

        # If the code reaches this point, either the timeout has been forced, or the target tx has been allowed to fail
        tx_escrow_timeout = ibcemulator.timeout(
            ibc_target_contract,
            ibc_packet,
            {"from": ibc_gov},
        )

        # # Check security limit remains unchanged (note that it may have decayed since the tx was mined)
        # assert abs(sp1.getUnitCapacity() - get_expected_decayed_units_capacity(
        #     ref_capacity            = init_sp1_unit_capacity,
        #     ref_capacity_timestamp  = init_timestamp,
        #     change_capacity_delta   = 0,
        #     change_timestamp        = tx_escrow_timeout.timestamp,
        #     current_timestamp       = chain[-1].timestamp,
        #     max_capacity            = sp1._max_unit_inflow()
        # )) <= 1 # Only allow for a very small rounding error

        # Escrow check
        assert sp1._escrowedTokens(from_token) == init_sp1_escrowed_balance - swap_amount # Escrowed balance has decreased
        assert sp1._escrowedFor(messageHash) == ZERO_ADDRESS  # Escrow info has been deleted

        # Escrow event
        escrow_timeout_events = tx_escrow_timeout.events['EscrowTimeout']
        assert len(escrow_timeout_events) == 1

        escrow_timeout_event = escrow_timeout_events[0]
        assert escrow_timeout_event['messageHash']   == messageHash
        assert escrow_timeout_event['liquiditySwap'] == False

        # Check balances
        assert from_token.balanceOf(fallback_user) == init_fallback_user_balance + swap_amount
        assert from_token.balanceOf(sp1)           == init_sp1_balance - swap_amount

        return RunFinishSwapResult(
            tx_swap_from_units  = None,
            tx_escrow_ack       = None,
            tx_escrow_timeout   = tx_escrow_timeout,
            output              = None,
            revert_exception    = revert_exception
        )



def get_swappool_spot_prices(swappool, tokens):

    # Computes for every pair of tokens 'a' and 'b' in the swappool:
    #   (b^amp · W_a) / (a^amp · W_b)

    spot_prices = []

    token_balances = [token.balanceOf(swappool) for token in tokens]
    token_weights  = [swappool._weight(token) for token in tokens]

    try:
        swappool_amp_f = swappool._amp() / 2**64
    except:
        swappool_amp_f = 1  # _amp does not exist in a non-amplified pool

    for a_idx, a_balance in enumerate(token_balances):
        for b_idx, b_balance in enumerate(token_balances[a_idx+1:]):
            spot_prices.append(
                (b_balance**swappool_amp_f * token_weights[a_idx]) / (a_balance**swappool_amp_f * token_weights[a_idx+1:][b_idx])
            )
    
    return spot_prices
