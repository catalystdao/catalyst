import pytest
from brownie import chain, convert, reverts
from brownie.test import given, strategy

pytestmark = pytest.mark.usefixtures("connect_pools", "finish_setup")

def test_security_limit_init(swappool1, swappool1_config, swappool2, swappool2_config):
    """
    Make sure the security limit gets correctly initialized
    """

    sp1_expected_max_capacity = sum([weight << 64 for weight in swappool1_config.get("weights")])

    assert sp1_expected_max_capacity == swappool1._max_unit_inflow()
    assert sp1_expected_max_capacity == swappool1.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity


    sp2_expected_max_capacity = sum([weight << 64 for weight in swappool2_config.get("weights")])

    assert sp2_expected_max_capacity == swappool2._max_unit_inflow()
    assert sp2_expected_max_capacity == swappool2.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity


@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=2000*10**18))
def test_security_limit_swap_loop(
    channelId,
    swappool1,
    swappool2,
    token1,
    token3,
    berg,
    deployer,
    ibcemulator,
    compute_expected_units_capacity,
    assert_abs_relative_error,
    swap_amount
):

    token1.transfer(berg, swap_amount, {'from': deployer})

    # Cache max capacities for speed
    sp1_max_capacity = swappool1._max_unit_inflow()
    sp2_max_capacity = swappool2._max_unit_inflow()


    # Swap from swappool1 to swappool2
    # 1. swapToUnits
    token1.approve(swappool1, swap_amount, {'from': berg})
    tx = swappool1.swapToUnits(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        0,  # Equal to False, False
        berg,
        {"from": berg},
    )
    tx_units = tx.events["SwapToUnits"]["output"]

    # 2. swapFromUnits
    if swappool2.getUnitCapacity() < tx_units:
        with reverts("Swap exceeds maximum swap amount. Please wait"):
            txe = ibcemulator.execute(
                tx.events["IncomingMetadata"]["metadata"][0],
                tx.events["IncomingPacket"]["packet"],
                {"from": berg}
            )
        return
    else:
        txe = ibcemulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg}
        )

    purchased_tokens = txe.events["SwapFromUnits"]["output"]

    # 3. Ack
    ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )

    
    # Verify security limit
    sp1_capacity = swappool1.getUnitCapacity()
    sp2_capacity = swappool2.getUnitCapacity()
    capacity_timestamp = chain[-1].timestamp

    assert sp1_capacity == sp1_max_capacity         # Will not change, since already at max capacity

    if tx_units > 0:                                # Explicit check to make sure the capacity of the second pool has decreased (if units have been transferred)
        assert sp2_capacity < sp2_max_capacity

    # Verify limit calculation
    assert abs(
        sp2_capacity -
        compute_expected_units_capacity(
            ref_capacity           = sp2_max_capacity,
            ref_capacity_timestamp = chain[0].timestamp,    # Doesn't really matter, as the limit is already at max capacity
            change_timestamp       = txe.timestamp,
            change_capacity_delta  = -tx_units,
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = sp2_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors

    


    # Reverse-swap from swappool2 to swappool1
    # 4. swapToUnits
    token3.approve(swappool2, purchased_tokens, {'from': berg})
    tx2 = swappool2.swapToUnits(
        channelId,
        convert.to_bytes(swappool1.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token3,
        0,
        purchased_tokens,
        0,
        0,  # Equal to False, False
        berg,
        {"from": berg},
    )
    tx2_units = tx2.events["SwapToUnits"]["output"]

    # Make sure the security limit of the source pool does not change before the ack
    assert swappool2.getUnitCapacity() == sp2_capacity

    # 5. swapFromUnits
    txe2 = ibcemulator.execute(tx2.events["IncomingMetadata"]["metadata"][0], tx2.events["IncomingPacket"]["packet"], {"from": berg})

    # 6. Ack
    ibcemulator.ack(
        tx2.events["IncomingMetadata"]["metadata"][0],
        tx2.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )


    # The security limit of swappool2 should have to return to (almost) max capacity
    assert_abs_relative_error(
        swappool2.getUnitCapacity(),
        sp2_max_capacity,
        error_bound=1e-4
    )

    # Verify security limit calculations
    assert abs(
        swappool1.getUnitCapacity() -
        compute_expected_units_capacity(
            ref_capacity           = sp1_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = txe2.timestamp,
            change_capacity_delta  = -tx2_units,
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = sp1_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors

    assert abs(
        swappool2.getUnitCapacity() -
        compute_expected_units_capacity(
            ref_capacity           = sp2_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = txe2.timestamp,
            change_capacity_delta  = tx2_units,
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = sp2_max_capacity,
        )
    ) <= 1      # Allow 1 unit for rounding errors
    



def test_security_limit_swap_timeout(
    channelId,
    swappool1,
    swappool2,
    token1,
    token3,
    berg,
    deployer,
    ibcemulator,
    compute_expected_units_capacity
):
    
    # Send a quarter of swappool1 token1 balance
    swap_amount =  int(token1.balanceOf(swappool1)/4)

    token1.transfer(berg, swap_amount, {'from': deployer})



    # Swap from swappool1 to swappool2 to introduce a change in the security limit of swappool2
    # 1. swapToUnits
    token1.approve(swappool1, swap_amount, {'from': berg})
    tx = swappool1.swapToUnits(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        0,  # Equal to False, False
        berg,
        {"from": berg},
    )

    # 2. swapFromUnits
    txe = ibcemulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg}
    )
    purchased_tokens = txe.events["SwapFromUnits"]["output"]

    # 3. Ack
    ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )



    # Cache the sp2 security limit capacity before the swap
    sp2_capacity = swappool2.getUnitCapacity()
    capacity_timestamp = chain[-1].timestamp

    # Reverse-swap from swappool2 to swappool1
    # 4. swapToUnits
    token3.approve(swappool2, purchased_tokens, {'from': berg})
    tx2 = swappool2.swapToUnits(
        channelId,
        convert.to_bytes(swappool1.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token3,
        0,
        purchased_tokens,
        0,
        0,  # Equal to False, False
        berg,
        {"from": berg},
    )

    # 6. Timeout
    ibcemulator.timeout(
        tx2.events["IncomingMetadata"]["metadata"][0],
        tx2.events["IncomingPacket"]["packet"],
        {"from": deployer}
    )

    # Make sure the security limit of the source pool does not change after the timeout (allow for the normal decay of the security limit)
    assert abs(
        swappool2.getUnitCapacity() - 
        compute_expected_units_capacity(
            ref_capacity           = sp2_capacity,
            ref_capacity_timestamp = capacity_timestamp,
            change_timestamp       = tx2.timestamp,
            change_capacity_delta  = 0,                         # ! Expected capacity change set to 0
            current_timestamp      = chain[-1].timestamp,
            max_capacity           = swappool2._max_unit_inflow(),
        )
    ) <= 1      # Allow 1 unit for rounding errors
