import pytest
from brownie import chain, convert, reverts
from brownie.test import given
from hypothesis.strategies import floats

from tests.catalyst.utils.pool_utils import compute_expected_units_capacity
from tests.catalyst.utils.common_utils import assert_abs_relative_error


pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_pools")

# TODO split into different tests?
@pytest.mark.no_call_coverage
@given(swap_amount_percentage=floats(min_value=0, max_value=2))    # From 0 to 2x the tokens hold by the pool
def test_security_limit_swap_loop(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    target_pool_tokens,
    berg,
    deployer,
    ibc_emulator,
    swap_amount_percentage
):
    source_token = source_pool_tokens[0]
    target_token = target_pool_tokens[0]

    swap_amount = int(swap_amount_percentage * source_token.balanceOf(source_pool))

    assert source_token.balanceOf(berg) == 0


    # Cache max capacities for speed
    source_max_capacity = source_pool._max_unit_inflow()
    target_max_capacity = target_pool._max_unit_inflow()


    # Swap from source_pool to target_pool
    # 1. swapToUnits
    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(source_pool, swap_amount, {'from': berg})
    tx = source_pool.swapToUnits(
        channel_id,
        convert.to_bytes(target_pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        source_token,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    tx_units = tx.events["SwapToUnits"]["output"]

    # 2. swapFromUnits
    if target_pool.getUnitCapacity() < tx_units:
        with reverts("Swap exceeds security limit"):
            txe = ibc_emulator.execute(
                tx.events["IncomingMetadata"]["metadata"][0],
                tx.events["IncomingPacket"]["packet"],
                {"from": berg}
            )
        return
    else:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg}
        )

    purchased_tokens = txe.events["SwapFromUnits"]["output"]

    # 3. Ack
    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )


    # Verify the security limit
    source_capacity = source_pool.getUnitCapacity()
    target_capacity = target_pool.getUnitCapacity()
    capacity_timestamp = chain[-1].timestamp

    assert source_capacity == source_max_capacity       # Will not change, since already at max capacity

    if tx_units > 0:                                    # Explicit check to make sure the capacity of the second pool has decreased (if units have been transferred)
        assert target_capacity < target_max_capacity

    # Verify target_capacity decrease calculation
    assert abs(
        target_capacity -
        compute_expected_units_capacity(
            ref_capacity           = target_max_capacity,
            ref_capacity_timestamp = chain[0].timestamp,    # Doesn't really matter, as the limit is already at max capacity
            change_timestamp       = txe.timestamp,
            change_capacity_delta  = -tx_units,
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = target_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors

    

    # Reverse-swap from target_pool to source_pool
    # 4. swapToUnits
    target_token.approve(target_pool, purchased_tokens, {'from': berg})
    tx2 = target_pool.swapToUnits(
        channel_id,
        convert.to_bytes(source_pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        target_token,
        0,
        purchased_tokens,
        0,
        berg,
        {"from": berg},
    )
    tx2_units = tx2.events["SwapToUnits"]["output"]

    
    # Make sure the security limit of the target pool does not change before the ack. NOTE: the unit capacity may have increased
    # slightly because of the time elapsed since the last capacity snapshot.

    # Intuitive check (unit capacity should not have changed much)
    assert_abs_relative_error(target_pool.getUnitCapacity(), target_capacity, 1e-2)
    # Accurate check
    assert abs(
        target_pool.getUnitCapacity() -
        compute_expected_units_capacity(
            ref_capacity           = target_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = tx2.timestamp,         # Doesn't really matter, as there has been no change
            change_capacity_delta  = 0,                     # ! No change expected
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = target_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors

    # 5. swapFromUnits
    txe2 = ibc_emulator.execute(tx2.events["IncomingMetadata"]["metadata"][0], tx2.events["IncomingPacket"]["packet"], {"from": berg})

    # 6. Ack
    ibc_emulator.ack(
        tx2.events["IncomingMetadata"]["metadata"][0],
        tx2.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )


    # Verify security limit calculations
    assert abs(
        source_pool.getUnitCapacity() -
        compute_expected_units_capacity(
            ref_capacity           = source_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = txe2.timestamp,
            change_capacity_delta  = -tx2_units,
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = source_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors


    # The security limit of target_pool should have to return to (almost) max capacity
    # Intuitive check
    assert_abs_relative_error(
        target_pool.getUnitCapacity(),
        target_max_capacity,
        error_bound=1e-4
    )
    # Accurate check
    assert abs(
        target_pool.getUnitCapacity() -
        compute_expected_units_capacity(
            ref_capacity           = target_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = txe2.timestamp,
            change_capacity_delta  = tx2_units,
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = target_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors
    



def test_security_limit_swap_timeout(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    target_pool_tokens,
    berg,
    deployer,
    ibc_emulator
):
    source_token = source_pool_tokens[0]
    target_token = target_pool_tokens[0]
    
    # Send a quarter of source_pool source_token balance
    swap_amount =  int(source_token.balanceOf(source_pool)/4)

    source_token.transfer(berg, swap_amount, {'from': deployer})



    # TODO create a fixture for this?
    # Swap from source_pool to target_pool to introduce a change in the security limit of target_pool
    # 1. swapToUnits
    source_token.approve(source_pool, swap_amount, {'from': berg})
    tx = source_pool.swapToUnits(
        channel_id,
        convert.to_bytes(target_pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        source_token,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )

    # 2. swapFromUnits
    txe = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg}
    )
    purchased_tokens = txe.events["SwapFromUnits"]["output"]

    # 3. Ack
    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )



    # Cache the target security limit capacity before the swap
    target_capacity = target_pool.getUnitCapacity()
    capacity_timestamp = chain[-1].timestamp

    # Reverse-swap from target_pool to source_pool
    # 4. swapToUnits
    target_token.approve(target_pool, purchased_tokens, {'from': berg})
    tx2 = target_pool.swapToUnits(
        channel_id,
        convert.to_bytes(source_pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        target_token,
        0,
        purchased_tokens,
        0,
        berg,
        {"from": berg},
    )

    # 6. Timeout
    ibc_emulator.timeout(
        tx2.events["IncomingMetadata"]["metadata"][0],
        tx2.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )

    # Make sure the security limit of the source pool does not change after the timeout (allow for the normal decay of the security limit)
    assert abs(
        target_pool.getUnitCapacity() - 
        compute_expected_units_capacity(
            ref_capacity           = target_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = tx2.timestamp,
            change_capacity_delta  = 0,                         # ! Expected capacity change set to 0
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = target_pool._max_unit_inflow(),
        )
    ) <= 1      # Allow 1 unit for rounding errors
