import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
from brownie.test import given, strategy
from hypothesis import example, settings

from utils.pool_utils import compute_liquidity_swap_hash

pytestmark = [
    pytest.mark.usefixtures("pool_connect_itself"),
    pytest.mark.no_pool_param
]

@pytest.mark.no_call_coverage
@example(swap_percentage=4000)
@given(swap_percentage=strategy("uint256", max_value=10000))
def test_ibc_ack(pool, channel_id, ibc_emulator, berg, deployer, swap_percentage):
    swap_amount = int(pool.totalSupply() * swap_percentage / 100000)
    
    pool.transfer(berg, swap_amount, {'from': deployer})
    assert pool._escrowedPoolTokens() == 0

    tx = pool.sendLiquidity(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        swap_amount,
        [0, 0],
        berg,
        {"from": berg},
    )
    userBalance = pool.balanceOf(berg)
    assert pool._escrowedPoolTokens() == swap_amount
    
    txe = ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert pool.balanceOf(berg) == userBalance
    assert pool._escrowedPoolTokens() == 0


@pytest.mark.no_call_coverage
@example(swap_percentage=4000)
@given(swap_percentage=strategy("uint256", max_value=10000))
def test_ibc_timeout(pool, channel_id, ibc_emulator, berg, deployer, swap_percentage):
    swap_amount = int(pool.totalSupply() * swap_percentage / 100000)
    
    pool.transfer(berg, swap_amount, {'from': deployer})
    assert pool._escrowedPoolTokens() == 0

    tx = pool.sendLiquidity(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        swap_amount,
        [0, 0],
        berg,
        {"from": berg},
    )
    userBalance = pool.balanceOf(berg)
    assert pool._escrowedPoolTokens() == swap_amount
    
    txe = ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert pool.balanceOf(berg) == swap_amount + userBalance
    assert pool._escrowedPoolTokens() == 0


def test_only_one_response(pool, channel_id, ibc_emulator, berg, deployer):
    swap_percentage = 10000
    swap_amount = int(pool.totalSupply() * swap_percentage / 100000)
    
    pool.transfer(berg, swap_amount, {'from': deployer})

    tx = pool.sendLiquidity(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        swap_amount,
        [0, 0],
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
            convert.to_bytes(0, "bytes"),   
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )
    
    chain.undo(3)
    
    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
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
            convert.to_bytes(0, "bytes"),
            tx.events["IncomingPacket"]["packet"],
            {"from": deployer},
        )


def test_ibc_ack_event(pool, channel_id, ibc_emulator, berg, deployer):
    """
        Test the SendLiquidityAck event gets fired.
    """
    swap_percentage = 10000
    swap_amount = int(pool.totalSupply() * swap_percentage / 100000)
    
    pool.transfer(berg, swap_amount, {'from': deployer})

    tx = pool.sendLiquidity(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        swap_amount,
        [0, 0],
        berg,
        {"from": berg},
    )
    
    txe = ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    ack_event = txe.events['SendLiquidityAck']


    expected_message_hash = compute_liquidity_swap_hash(
        berg.address,
        tx.return_value,
        swap_amount,
        tx.block_number
    )

    assert ack_event["swapHash"] == expected_message_hash


def test_ibc_timeout_event(pool, channel_id, ibc_emulator, berg, deployer):
    """
        Test the SendLiquidityTimeout event gets fired.
    """
    swap_percentage = 10000
    swap_amount = int(pool.totalSupply() * swap_percentage / 100000)
    
    pool.transfer(berg, swap_amount, {'from': deployer})

    tx = pool.sendLiquidity(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        convert.to_bytes(berg.address.replace("0x", "")),
        swap_amount,
        [0, 0],
        berg,
        {"from": berg},
    )
    
    txe = ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    timeout_event = txe.events['SendLiquidityTimeout']

    expected_message_hash = compute_liquidity_swap_hash(
        berg.address,
        tx.return_value,
        swap_amount,
        tx.block_number
    )

    assert timeout_event["swapHash"] == expected_message_hash
