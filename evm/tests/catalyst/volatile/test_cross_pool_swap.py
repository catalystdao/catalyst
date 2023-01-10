import brownie
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts
from brownie.test import given, strategy

pytestmark = pytest.mark.usefixtures("connect_pools", "finish_setup")


@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=2000*10**18))
def test_cross_pool_swap(channelId, swappool1, swappool2, token1, token3, berg, deployer, compute_expected_swap, ibcemulator, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token3, swappool1, swappool2)
    
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
    assert token1.balanceOf(berg) == 0
    
    if swappool2.getUnitCapacity() < tx.events["SwapToUnits"]["output"]:
        with reverts("Swap exceeds maximum swap amount. Please wait"):
            txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromUnits"]["output"]
    
    assert purchased_tokens == token3.balanceOf(berg)
    
    if swap_amount/(token1.balanceOf(swappool1)-swap_amount) < 1e-06:
        assert purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"
    else:
        assert purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"
        assert (y * 9 /10) <= purchased_tokens, "Swap returns less than 9/10 theoretical"



@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=10**17))
def test_cross_pool_swap_approx(channelId, swappool1, swappool2, token1, token3, berg, deployer, compute_expected_swap, ibcemulator, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token3, swappool1, swappool2)
    
    tx = swappool1.swapToUnits(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        3,  # Equal to True, True
        berg,
        {"from": berg},
    )
    assert token1.balanceOf(berg) == 0
    
    # Given that we are leading with small swap amounts, the security limit should never be reached
    txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromUnits"]["output"]
    
    assert purchased_tokens == token3.balanceOf(berg)
    
    if swap_amount/(token1.balanceOf(swappool1)-swap_amount) < 1e-06:
        assert purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"
    else:
        assert purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"
        assert (y * 9 /10) <= purchased_tokens, "Swap returns less than 9/10 theoretical"



@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=10*10**18))
def test_cross_pool_swap_min_out(channelId, swappool1, swappool2, token1, token3, berg, deployer, compute_expected_swap, ibcemulator, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token3, swappool1, swappool2)
    min_out = int(y * 1.2)  # Make sure the swap always fails
    
    tx = swappool1.swapToUnits(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        min_out,
        0,  # Equal to False, False
        berg,
        {"from": berg},
    )
    assert token1.balanceOf(berg) == 0

    if min_out == 0:
        return

    with brownie.reverts("Insufficient Return"):
        ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
