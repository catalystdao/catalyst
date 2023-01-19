import pytest
from brownie import reverts, convert
from brownie.test import given, strategy
from hypothesis.strategies import floats

pytestmark = pytest.mark.usefixtures("pool_connect_itself")

@pytest.mark.no_call_coverage
@given(swap_percentage=floats(min_value=0, max_value=2))    # From 0 to 2x the tokens hold by the pool
def test_self_swap(
    pool,
    pool_tokens,
    berg,
    deployer,
    channel_id,
    ibc_emulator,
    swap_percentage
):
    token = pool_tokens[0]
    swap_amount = int(swap_percentage * token.balanceOf(pool))

    assert token.balanceOf(berg) == 0

    token.transfer(berg, swap_amount, {'from': deployer})
    token.approve(pool, swap_amount, {'from': berg})
    
    tx = pool.swapToUnits(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert token.balanceOf(berg) == 0
    
    if pool.getUnitCapacity() < tx.events["SwapToUnits"]["output"]:
        with reverts("Swap exceeds maximum swap amount. Please wait"):
            txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromUnits"]["output"]
    
    assert token.balanceOf(berg) == purchased_tokens
    
    # We don't check that the swap returns less than a certain threshold because the escrow functionality impacts how close the swap can actually get to 1:1. Also, it should always return less than the input. INcluding errors.
    assert purchased_tokens <= swap_amount, "Swap returns more than theoretical"