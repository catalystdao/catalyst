import pytest
from brownie import reverts, convert, web3
from brownie.test import given, strategy


pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_pools")

#TODO add fees test (create fixture that sets up non-zero fees to the pool)

@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=2000*10**18))
def test_cross_pool_swap(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    target_pool_tokens,
    berg,
    deployer,
    ibc_emulator,
    compute_expected_swap,
    swap_amount
):
    #TODO parametrize source_token and target_tokens?
    source_token = source_pool_tokens[0]
    target_token = target_pool_tokens[0]

    assert target_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(source_pool, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, source_token, target_token)['output']
    
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
    assert source_token.balanceOf(berg) == 0
    
    # The swap may revert because of the security limit     #TODO mark these cases as 'skip'?
    if target_pool.getUnitCapacity() < tx.events["SwapToUnits"]["output"]:
        with reverts("Swap exceeds security limit. Please wait"):
            txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromUnits"]["output"]
    
    assert purchased_tokens == target_token.balanceOf(berg)

    # Make sure no more than it's theoretically due is given
    assert purchased_tokens <= y + 1 or purchased_tokens <= int(y*1.000001), "Swap returns more than theoretical"       #TODO set bound as test variable?

    # Make sure no much less than it's theoretically due is given (ignore check for very small swaps)
    if swap_amount > 1000:
        assert purchased_tokens >= y - 1 or purchased_tokens >= int(y*9/10), "Swap returns much less than theoretical"  #TODO set bound as test variable?



@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=10*10**18))
def test_cross_pool_swap_min_out(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    target_pool_tokens,
    berg,
    deployer,
    compute_expected_swap,
    ibc_emulator,
    swap_amount
):
    
    source_token = source_pool_tokens[0]
    target_token = target_pool_tokens[0]

    assert target_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(source_pool, swap_amount, {'from': berg})
    
    y = compute_expected_swap(swap_amount, source_token, target_token)['output']
    min_out = int(y * 1.2)  # Make sure the swap always fails
    
    tx = source_pool.swapToUnits(
        channel_id,
        convert.to_bytes(target_pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        source_token,
        0,
        swap_amount,
        min_out,
        berg,
        {"from": berg},
    )

    assert source_token.balanceOf(berg) == 0

    if min_out == 0:
        return

    with reverts("Insufficient Return"):
        ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})


def test_swap_to_units_event(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    berg,
    elwood,
    deployer
):
    """
        Test the SwapToUnits event gets fired.
    """

    swap_amount = 10**8
    min_out     = 100
    
    source_token = source_pool_tokens[0]

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(source_pool, swap_amount, {'from': berg})
    
    tx = source_pool.swapToUnits(
        channel_id,
        convert.to_bytes(target_pool.address.replace("0x", "")),
        convert.to_bytes(elwood.address.replace("0x", "")),     # NOTE: not using the same account as the caller of the tx to make sure the 'targetUser' is correctly reported
        source_token,
        1,                                                      # NOTE: use non-zero target asset index to make sure the field is set on the event (and not just left blank)
        swap_amount,
        min_out,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value
    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    swap_to_units_event = tx.events['SwapToUnits']

    assert swap_to_units_event['targetPool']   == target_pool
    assert swap_to_units_event['targetUser']   == elwood
    assert swap_to_units_event['fromAsset']    == source_token
    assert swap_to_units_event['toAssetIndex'] == 1
    assert swap_to_units_event['input']        == swap_amount
    assert swap_to_units_event['output']       == observed_units
    assert swap_to_units_event['minOut']       == min_out
    assert swap_to_units_event['messageHash']  == expected_message_hash


def test_swap_from_units_event(
    channel_id,
    source_pool,
    target_pool,
    source_pool_tokens,
    target_pool_tokens,
    berg,
    elwood,
    deployer,
    ibc_emulator
):
    """
        Test the SwapToUnits event gets fired.
    """

    swap_amount = 10**8

    source_token = source_pool_tokens[0]
    target_token = target_pool_tokens[0]

    source_token.transfer(berg, swap_amount, {'from': deployer})
    source_token.approve(source_pool, swap_amount, {'from': berg})
    
    tx = source_pool.swapToUnits(
        channel_id,
        convert.to_bytes(target_pool.address.replace("0x", "")),
        convert.to_bytes(elwood.address.replace("0x", "")),
        source_token,
        0,
        swap_amount,
        0,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value
    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})

    swap_from_units_event = txe.events['SwapFromUnits']

    assert swap_from_units_event['who']         == elwood
    assert swap_from_units_event['toAsset']     == target_token
    assert swap_from_units_event['input']       == observed_units
    assert swap_from_units_event['output']      == target_token.balanceOf(elwood)
    assert swap_from_units_event['messageHash'] == expected_message_hash
