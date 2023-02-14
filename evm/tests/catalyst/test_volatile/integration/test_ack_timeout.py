import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
from brownie.test import given, strategy
from hypothesis import settings

from utils.pool_utils import compute_asset_swap_hash

pytestmark = pytest.mark.usefixtures("pool_connect_itself")

@pytest.mark.no_call_coverage
@given(swap_percentage=strategy("uint256", max_value=100000))
def test_ibc_ack(pool, channel_id, pool_tokens, ibc_emulator, berg, deployer, swap_percentage):
    swap_amount = int(pool_tokens[0].balanceOf(deployer) * swap_percentage / 100000)
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})

    tx = pool.sendSwap(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool_tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    userBalance = pool_tokens[0].balanceOf(berg)
    
    txe = ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert pool_tokens[0].balanceOf(berg) == userBalance


@pytest.mark.no_call_coverage
@given(swap_percentage=strategy("uint256", max_value=100000))
def test_ibc_timeout(pool, channel_id, pool_tokens, ibc_emulator, berg, deployer, swap_percentage):
    swap_amount = int(pool_tokens[0].balanceOf(deployer) * swap_percentage / 100000)
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})

    tx = pool.sendSwap(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool_tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    userBalance = pool_tokens[0].balanceOf(berg)
    
    txe = ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert pool_tokens[0].balanceOf(berg) == swap_amount + userBalance


def test_only_one_response(pool, channel_id, pool_tokens, ibc_emulator, berg, deployer):
    swap_amount = pool_tokens[0].balanceOf(deployer) / 10
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})

    tx = pool.sendSwap(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool_tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    
    ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    
    with reverts():
        ibc_emulator.timeout(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    with reverts():
        ibc_emulator.ack(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    chain.undo(3)
    
    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    
    with reverts():
        ibc_emulator.timeout(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    with reverts():
        ibc_emulator.ack(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )


@given(swap_percentage=strategy("uint256", max_value=100000, min_value=10))
def test_ibc_timeout_and_ack(pool, channel_id, pool_tokens, ibc_emulator, berg, deployer, swap_percentage):
    if len(pool_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a pool to run a local swap.")
    swap_amount = int(pool_tokens[0].balanceOf(deployer) * swap_percentage / 100000)
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})
    
    token1 = pool_tokens[0]
    token2 = pool_tokens[1]

    U = int(693147180559945344 / 2)  # Example value used to test if the swap is corrected.

    both1_12 = pool.calcLocalSwap(token1, token2, 10**18)
    both1_21 = pool.calcLocalSwap(token2, token1, 10**18)
    to1 = pool.calcSendSwap(token1, 10**18)
    from1 = pool.calcReceiveSwap(token1, U)

    tx1 = pool.sendSwap(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        token1,
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )

    both2_12 = pool.calcLocalSwap(token1, token2, 10**18)
    both2_21 = pool.calcLocalSwap(token2, token1, 10**18)
    to2 = pool.calcSendSwap(token1, 10**18)
    from2 = pool.calcReceiveSwap(token1, U)

    assert both1_12 > both2_12
    assert both1_21 == both2_21
    assert to1 > to2
    assert from1 == from2

    txe = ibc_emulator.timeout(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = pool.calcLocalSwap(token1, token2, 10**18)
    both3_21 = pool.calcLocalSwap(token2, token1, 10**18)
    to3 = pool.calcSendSwap(token1, 10**18)
    from3 = pool.calcReceiveSwap(token1, U)

    assert both1_12 == both3_12
    assert both1_21 == both3_21
    assert to1 == to3
    assert from1 == from3

    chain.undo()

    txe = ibc_emulator.ack(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = pool.calcLocalSwap(token1, token2, 10**18)
    both3_21 = pool.calcLocalSwap(token2, token1, 10**18)
    to3 = pool.calcSendSwap(token1, 10**18)
    from3 = pool.calcReceiveSwap(token1, U)

    assert both1_12 > both3_12
    assert both1_21 < both3_21
    assert to1 > to3
    assert from1 < from3


def test_ibc_ack_event(pool, channel_id, pool_tokens, ibc_emulator, berg, deployer):
    """
        Test the EscrowAck event gets fired.
    """

    swap_amount = pool_tokens[0].balanceOf(deployer) / 1000
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})

    tx = pool.sendSwap(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool_tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    userBalance = pool_tokens[0].balanceOf(berg)
    
    txe = ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    escrow_ack_event = txe.events['EscrowAck']


    expected_message_hash = compute_asset_swap_hash(
        berg,
        tx.return_value,
        swap_amount,
        pool_tokens[0],
        tx.block_number
    )

    assert escrow_ack_event["messageHash"]   == expected_message_hash
    assert escrow_ack_event["liquiditySwap"] == False


def test_ibc_timeout_event(pool, channel_id, pool_tokens, ibc_emulator, berg, deployer):
    """
        Test the EscrowTimeout event gets fired.
    """

    swap_amount = pool_tokens[0].balanceOf(deployer) / 1000
    
    pool_tokens[0].transfer(berg, swap_amount, {'from': deployer})
    pool_tokens[0].approve(pool, swap_amount, {'from': berg})

    tx = pool.sendSwap(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        pool_tokens[0],
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    
    txe = ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    escrow_timeout_event = txe.events['EscrowTimeout']


    expected_message_hash = compute_asset_swap_hash(
        berg,
        tx.return_value,
        swap_amount,
        pool_tokens[0],
        tx.block_number
    )

    assert escrow_timeout_event["messageHash"]   == expected_message_hash
    assert escrow_timeout_event["liquiditySwap"] == False
