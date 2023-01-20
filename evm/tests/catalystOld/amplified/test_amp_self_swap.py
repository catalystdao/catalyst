import brownie
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts
from brownie.test import given, strategy

pytestmark = pytest.mark.usefixtures("connect_pools", "finish_setup")


@pytest.mark.no_call_coverage
@given(swap_percentage=strategy("uint256", max_value=10000))
def test_cross_pool_swap(channelId, swappool_amp, token1, berg, deployer, ibcemulator, swap_percentage):
    swap_amount = token1.balanceOf(swappool_amp)*swap_percentage // 10000
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool_amp, swap_amount, {'from': berg})
    
    y = swap_amount
    
    tx = swappool_amp.swapToUnits(
        channelId,
        convert.to_bytes(swappool_amp.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert token1.balanceOf(berg) == 0
    
    if swappool_amp.getUnitCapacity() < tx.events["SwapToUnits"]["output"]:
        with reverts("Swap exceeds security limit"):
            txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromUnits"]["output"]
    
    assert purchased_tokens == token1.balanceOf(berg)
    
    # We don't check that the swap returns less than a certain threshold because the escrow functionality impacts how close the swap can actually get to 1:1. Also, it should always return less than the input. INcluding errors.
    assert purchased_tokens <= swap_amount, "Swap returns more than theoretical"
    