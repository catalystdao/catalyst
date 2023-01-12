import brownie
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
from brownie.test import given, strategy

pytestmark = pytest.mark.usefixtures("connect_pools", "finish_setup")


@pytest.mark.no_call_coverage
@given(deposit_percentage=strategy("uint256", max_value=20000), swap_percentage=strategy("uint256", max_value=10000))
def test_liquidity_swap(channelId, swappool1, swappool2, berg, deployer, compute_expected_liquidity_swap, ibcemulator, get_pool_tokens, swap_percentage, deposit_percentage):
    swap_percentage /= 10000
    deposit_percentage /= 10000
    
    tokens1 = get_pool_tokens(swappool1)
    tokens2 = get_pool_tokens(swappool2)
    
    deposit_amounts = [int(token.balanceOf(swappool1) * deposit_percentage) for token in tokens1]
    [token.transfer(berg, amount, {'from': deployer}) for amount, token in zip(deposit_amounts, tokens1)]
    [token.approve(swappool1, amount, {'from': berg}) for amount, token in zip(deposit_amounts, tokens1)]
    
    estimatedPoolTokens = int(swappool1.totalSupply()*deposit_percentage)
    
    tx = swappool1.depositMixed(deposit_amounts, int(estimatedPoolTokens*0.999), {"from": berg})
    
    pool1_tokens = tx.return_value
    
    pool1_tokens_swapped = int(pool1_tokens * swap_percentage)
    
    U, estimatedPool2Tokens = compute_expected_liquidity_swap(pool1_tokens_swapped, swappool1, swappool2, withU=True)
    
    tx = swappool1.outLiquidity(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool1_tokens_swapped,
        int(estimatedPool2Tokens*9/10),
        berg,
        {"from": berg}
    )
    assert swappool1.balanceOf(berg) == pool1_tokens - pool1_tokens_swapped
    
    if swappool2.getUnitCapacity() < tx.events["SwapToLiquidityUnits"]["output"]:
        with reverts("Swap exceeds maximum swap amount. Please wait"):
            txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
        return
    else:
        txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    purchased_tokens = txe.events["SwapFromLiquidityUnits"]["output"]
    
    assert purchased_tokens == swappool2.balanceOf(berg)
    
    if swap_percentage < 1e-05:
        assert purchased_tokens <= int(estimatedPool2Tokens*1.000001), "Swap returns more than theoretical"
    else:
        assert purchased_tokens <= int(estimatedPool2Tokens*1.000001), "Swap returns more than theoretical"
        assert (estimatedPool2Tokens * 9 /10) <= purchased_tokens, "Swap returns less than 9/10 theoretical"
    


@pytest.mark.parametrize("approx", [0, 1, 2, 3], ids=["no-approx", "swap-to-approx", "swap-from-approx", "all-approx"])
def test_swap_to_liquidity_units_event(channelId, swappool1, swappool1_config, swappool2, berg, elwood, approx):
    """
        Test the SwapToLiquidityUnits event gets fired.
    """

    swap_amount = 10**8
    
    tx = swappool1.outLiquidity(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),       # NOTE: not using the same account as the caller of the tx to make sure the 'targetUser' is correctly reported
        swap_amount,
        0,
        approx,
        elwood,
        {"from": swappool1_config.get("deployer")}              # Using the deployer as the sender as it will already have some pool tokens
    )

    observed_liquidity_units = tx.return_value
    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    swap_to_units_event = tx.events['SwapToLiquidityUnits']

    assert swap_to_units_event['targetPool']  == swappool2
    assert swap_to_units_event['targetUser']  == berg
    assert swap_to_units_event['input']       == swap_amount
    assert swap_to_units_event['output']      == observed_liquidity_units
    assert swap_to_units_event['fees']        >= 0                         # Check there is a fees field
    assert swap_to_units_event['messageHash'] == expected_message_hash



@pytest.mark.parametrize("approx", [0, 1, 2, 3], ids=["no-approx", "swap-to-approx", "swap-from-approx", "all-approx"])
def test_swap_from_liquidity_units_event(channelId, swappool1, swappool1_config, swappool2, berg, elwood, ibcemulator, approx):
    """
        Test the SwapFromLiquidityUnits event gets fired.
    """

    swap_amount = 10**8
    
    tx = swappool1.outLiquidity(
        channelId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        swap_amount,
        0,
        approx,
        elwood,
        {"from": swappool1_config.get("deployer")}              # Using the deployer as the sender as it will already have some pool tokens
    )

    observed_liquidity_units = tx.return_value
    expected_message_hash    = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})

    swap_from_liquidity_units_event = txe.events['SwapFromLiquidityUnits']

    assert swap_from_liquidity_units_event['who']         == berg
    assert swap_from_liquidity_units_event['input']       == observed_liquidity_units
    assert swap_from_liquidity_units_event['output']      == swappool2.balanceOf(berg)
    assert swap_from_liquidity_units_event['messageHash'] == expected_message_hash
