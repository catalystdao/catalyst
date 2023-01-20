import brownie
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
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
        berg,
        {"from": berg},
    )
    assert token1.balanceOf(berg) == 0
    
    if swappool2.getUnitCapacity() < tx.events["SwapToUnits"]["output"]:
        with reverts("Swap exceeds security limit. Please wait"):
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
        berg,
        {"from": berg},
    )
    assert token1.balanceOf(berg) == 0

    if min_out == 0:
        return

    with brownie.reverts("Insufficient Return"):
        ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})


def test_swap_to_units_event(channelId, swappool1, swappool2, token1, berg, elwood, deployer):
    """
        Test the SwapToUnits event gets fired.
    """

    swap_amount = 10**8
    min_out     = 100

    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1, swap_amount, {'from': berg})
    
    tx = swappool1.swapToUnits(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(elwood.address.replace("0x", "")),     # NOTE: not using the same account as the caller of the tx to make sure the 'targetUser' is correctly reported
        token1,
        1,                                                      # NOTE: use non-zero target asset index to make sure the field is set on the event (and not just left blank)
        swap_amount,
        min_out,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value
    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    swap_to_units_event = tx.events['SwapToUnits']

    assert swap_to_units_event['targetPool']   == swappool2
    assert swap_to_units_event['targetUser']   == elwood
    assert swap_to_units_event['fromAsset']    == token1
    assert swap_to_units_event['toAssetIndex'] == 1
    assert swap_to_units_event['input']        == swap_amount
    assert swap_to_units_event['output']       == observed_units
    assert swap_to_units_event['minOut']       == min_out
    assert swap_to_units_event['messageHash']  == expected_message_hash


def test_swap_from_units_event(channelId, swappool1, swappool2, token1, token3, berg, elwood, deployer, ibcemulator):
    """
        Test the SwapToUnits event gets fired.
    """

    swap_amount = 10**8

    token1.transfer(berg, swap_amount, {'from': deployer})
    token1.approve(swappool1, swap_amount, {'from': berg})
    
    tx = swappool1.swapToUnits(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(elwood.address.replace("0x", "")),
        token1,
        0,
        swap_amount,
        0,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value
    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})

    swap_from_units_event = txe.events['SwapFromUnits']

    assert swap_from_units_event['who']         == elwood
    assert swap_from_units_event['toAsset']     == token3
    assert swap_from_units_event['input']       == observed_units
    assert swap_from_units_event['output']      == token3.balanceOf(elwood)
    assert swap_from_units_event['messageHash'] == expected_message_hash
