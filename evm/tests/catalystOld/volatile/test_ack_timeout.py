import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
from brownie.test import given, strategy
from hypothesis import settings


@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=1000*10**18))
def test_ibc_ack(channelId, swappool, ibcemulator, get_pool_tokens, berg, deployer, swap_amount):
    tokens = get_pool_tokens(swappool)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool, swap_amount, {'from': berg})

    tx = swappool.swapToUnits(
        channelId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(berg.address.replace("0x", "")),
        tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    userBalance = tokens[0].balanceOf(berg)
    
    txe = ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert tokens[0].balanceOf(berg) == userBalance


@pytest.mark.no_call_coverage
@given(swap_amount=strategy("uint256", max_value=1000*10**18))
def test_ibc_timeout(channelId, swappool, ibcemulator, get_pool_tokens, berg, deployer, swap_amount):
    tokens = get_pool_tokens(swappool)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool, swap_amount, {'from': berg})

    tx = swappool.swapToUnits(
        channelId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(berg.address.replace("0x", "")),
        tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    userBalance = tokens[0].balanceOf(berg)
    
    txe = ibcemulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert tokens[0].balanceOf(berg) == swap_amount + userBalance


def test_only_one_response(channelId, swappool, ibcemulator, get_pool_tokens, berg, deployer):
    swap_amount = 10 * 10**18
    tokens = get_pool_tokens(swappool)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool, swap_amount, {'from': berg})

    tx = swappool.swapToUnits(
        channelId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(berg.address.replace("0x", "")),
        tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    
    ibcemulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    
    with reverts():
        ibcemulator.timeout(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    with reverts():
        ibcemulator.ack(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    chain.undo(3)
    
    ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    
    with reverts():
        ibcemulator.timeout(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    with reverts():
        ibcemulator.ack(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )


@given(swap_amount=strategy("uint256", max_value=1000*10**18, min_value=10**14))
def test_ibc_timeout_and_ack(channelId, swappool, ibcemulator, get_pool_tokens, berg, deployer, swap_amount):
    tokens = get_pool_tokens(swappool)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool, swap_amount, {'from': berg})
    
    token1 = tokens[0]
    token2 = tokens[1]

    U = int(693147180559945344 / 2)  # Example value used to test if the swap is corrected.

    both1_12 = swappool.dry_swap_both(token1, token2, 10**18)
    both1_21 = swappool.dry_swap_both(token2, token1, 10**18)
    to1 = swappool.dry_swap_to_unit(token1, 10**18)
    from1 = swappool.dry_swap_from_unit(token1, U)

    tx1 = swappool.swapToUnits(
        channelId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )

    both2_12 = swappool.dry_swap_both(token1, token2, 10**18)
    both2_21 = swappool.dry_swap_both(token2, token1, 10**18)
    to2 = swappool.dry_swap_to_unit(token1, 10**18)
    from2 = swappool.dry_swap_from_unit(token1, U)

    assert both1_12 > both2_12
    assert both1_21 == both2_21
    assert to1 > to2
    assert from1 == from2

    txe = ibcemulator.timeout(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = swappool.dry_swap_both(token1, token2, 10**18)
    both3_21 = swappool.dry_swap_both(token2, token1, 10**18)
    to3 = swappool.dry_swap_to_unit(token1, 10**18)
    from3 = swappool.dry_swap_from_unit(token1, U)

    assert both1_12 == both3_12
    assert both1_21 == both3_21
    assert to1 == to3
    assert from1 == from3

    chain.undo()

    txe = ibcemulator.ack(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = swappool.dry_swap_both(token1, token2, 10**18)
    both3_21 = swappool.dry_swap_both(token2, token1, 10**18)
    to3 = swappool.dry_swap_to_unit(token1, 10**18)
    from3 = swappool.dry_swap_from_unit(token1, U)

    assert both1_12 > both3_12
    assert both1_21 < both3_21
    assert to1 > to3
    assert from1 < from3


def test_ibc_ack_event(channelId, swappool, ibcemulator, get_pool_tokens, berg, deployer):
    """
        Test the EscrowAck event gets fired.
    """

    swap_amount = 10**8

    tokens = get_pool_tokens(swappool)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool, swap_amount, {'from': berg})

    tx = swappool.swapToUnits(
        channelId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(berg.address.replace("0x", "")),
        tokens[0],
        1,
        swap_amount,
        0,
        0,
        berg,
        {"from": berg},
    )
    userBalance = tokens[0].balanceOf(berg)
    
    txe = ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    escrow_ack_event = txe.events['EscrowAck']

    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    assert escrow_ack_event["messageHash"]   == expected_message_hash
    assert escrow_ack_event["liquiditySwap"] == False


def test_ibc_timeout_event(channelId, swappool, ibcemulator, get_pool_tokens, berg, deployer):
    """
        Test the EscrowTimeout event gets fired.
    """

    swap_amount = 10**8

    tokens = get_pool_tokens(swappool)
    
    tokens[0].transfer(berg, swap_amount, {'from': deployer})
    tokens[0].approve(swappool, swap_amount, {'from': berg})

    tx = swappool.swapToUnits(
        channelId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(berg.address.replace("0x", "")),
        tokens[0],
        1,
        swap_amount,
        0,
        0,
        berg,
        {"from": berg},
    )
    
    txe = ibcemulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    escrow_timeout_event = txe.events['EscrowTimeout']

    expected_message_hash = web3.keccak(tx.events["IncomingPacket"]["packet"][3]).hex()   # Keccak of the payload contained on the ibc packet

    assert escrow_timeout_event["messageHash"]   == expected_message_hash
    assert escrow_timeout_event["liquiditySwap"] == False
