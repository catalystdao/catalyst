import pytest
from brownie import web3, reverts
from brownie.test import given, strategy
from hypothesis import example
from hypothesis.strategies import floats
import re
from utils.common_utils import convert_64_bytes_address

pytestmark = pytest.mark.usefixtures("pool_connect_itself")


def test_out_of_gas(
    pool,
    pool_tokens,
    berg,
    deployer,
    channel_id,
    ibc_emulator
):
    token = pool_tokens[0]
    swap_amount = int(0.5 * token.balanceOf(pool))

    token.transfer(berg, swap_amount, {'from': deployer})
    token.approve(pool, swap_amount, {'from': berg})
    
    tx = pool.sendAsset(
        channel_id,
        convert_64_bytes_address(pool.address),
        convert_64_bytes_address(berg.address),
        token,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert token.balanceOf(berg) == 0
    
    # Now estimate gas
    contract = web3.eth.contract(address=ibc_emulator.address, abi=ibc_emulator.abi)
    estimated_gas = contract.functions.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"]
    ).estimateGas({'from': berg.address})
    
    # It is very important that this call reverts and doesn't fail on ack.
    # as otherwise, relayers can cheap out on transactions and cause all swaps to fail.
    # Alternativly, it can be used as a form of censorship.
    with reverts():
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg, "gas_limit": int(estimated_gas/1.10)}
        )
        
        # Check if the acknowledgement message has been emitted.
        assert "Acknowledgement" not in txe.events.keys()
    
    # Executing with correct gas is fine.
    txe = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg, "gas_limit": estimated_gas}
    )
    
    assert txe.events["Acknowledgement"]["acknowledgement"] == hex(0x00)
        
    
    
    