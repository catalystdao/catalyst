import brownie
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts
from brownie.test import given, strategy

pytestmark = pytest.mark.usefixtures("connect_pools", "finish_setup")


@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=10*10**18))
def test_amp_cross_pool_swap(channelId, swappool1_amp, swappool2_amp, token1, token3, berg, deployer, compute_expected_swap, ibcemulator, swap_amount):
    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1_amp, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, token1, token3, swappool1_amp, swappool2_amp)
    
    tx = swappool1_amp.swapToUnits(
        channelId,
        convert.to_bytes(swappool2_amp.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert token1.balanceOf(berg) == 0
    
    if swappool2_amp.getUnitCapacity() < tx.events["SwapToUnits"]["output"]:
        with reverts("Swap exceeds maximum swap amount. Please wait"):
            txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromUnits"]["output"]
    
    assert purchased_tokens == token3.balanceOf(berg)
    
    if swap_amount/(token1.balanceOf(swappool1_amp)-swap_amount) < 1e-06:
        assert purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"
    else:
        assert purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"
        assert (y * 9 /10) <= purchased_tokens, "Swap returns less than 9/10 theoretical"


#TODO add test that empties the pool


def test_swap_to_units_event(channelId, swappool1_amp, swappool2_amp, token1, berg, elwood, deployer):
    """
        Test the SwapToUnits event gets fired.
    """

    swap_amount = 10**8
    min_out     = 100

    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1_amp, swap_amount, {'from': berg})
    
    tx = swappool1_amp.swapToUnits(
        channelId,
        convert.to_bytes(swappool2_amp.address.replace("0x", "")),
        convert.to_bytes(elwood.address.replace("0x", "")),
        token1,
        1,                                                      # NOTE: use non-zero target asset index to make sure the field is set on the event (and not just left blank)
        swap_amount,
        min_out,
        elwood,                                                 # NOTE: not using the same account as the caller of the tx to make sure the 'targetUser' is correctly reported
        {"from": berg},
    )

    observed_units = tx.return_value

    swap_to_units_event = tx.events['SwapToUnits']

    assert swap_to_units_event['targetPool']   == swappool2_amp
    assert swap_to_units_event['targetUser']   == elwood
    assert swap_to_units_event['fromAsset']    == token1
    assert swap_to_units_event['toAssetIndex'] == 1
    assert swap_to_units_event['input']        == swap_amount
    assert swap_to_units_event['output']       == observed_units
    assert swap_to_units_event['minOut']       == min_out

    assert swap_to_units_event['messageHash'] is not None



def test_swap_from_units_event(channelId, swappool1_amp, swappool2_amp, token1, token3, berg, elwood, deployer, ibcemulator):
    """
        Test the SwapToUnits event gets fired.
    """

    swap_amount = 10**8

    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1_amp, swap_amount, {'from': berg})
    
    tx = swappool1_amp.swapToUnits(
        channelId,
        convert.to_bytes(swappool2_amp.address.replace("0x", "")),
        convert.to_bytes(elwood.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value

    txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})

    swap_from_units_event = txe.events['SwapFromUnits']

    assert swap_from_units_event['who']     == elwood
    assert swap_from_units_event['toAsset'] == token3
    assert swap_from_units_event['input']   == observed_units
    assert swap_from_units_event['output']  == token3.balanceOf(elwood)

    assert swap_from_units_event['messageHash'] is not None