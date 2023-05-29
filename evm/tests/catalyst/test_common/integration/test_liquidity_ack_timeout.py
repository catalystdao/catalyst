import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain, convert, reverts, web3
from brownie.test import given, strategy
from hypothesis import example, settings

from utils.common_utils import convert_64_bytes_address
from utils.vault_utils import compute_liquidity_swap_hash

pytestmark = [
    pytest.mark.usefixtures("vault_connect_itself"),
    pytest.mark.no_vault_param,
]


@pytest.mark.no_call_coverage
@example(swap_percentage=4000)
@given(swap_percentage=strategy("uint256", max_value=10000))
def test_ibc_ack(vault, channel_id, ibc_emulator, berg, deployer, swap_percentage):
    swap_amount = int(vault.totalSupply() * swap_percentage / 100000)

    vault.transfer(berg, swap_amount, {"from": deployer})
    assert vault._escrowedVaultTokens() == 0

    tx = vault.sendLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        swap_amount,
        [0, 0],
        berg,
        {"from": berg},
    )
    userBalance = vault.balanceOf(berg)
    assert vault._escrowedVaultTokens() == swap_amount

    txe = ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert vault.balanceOf(berg) == userBalance
    assert vault._escrowedVaultTokens() == 0


@pytest.mark.no_call_coverage
@example(swap_percentage=4000)
@given(swap_percentage=strategy("uint256", max_value=10000))
def test_ibc_timeout(vault, channel_id, ibc_emulator, berg, deployer, swap_percentage):
    swap_amount = int(vault.totalSupply() * swap_percentage / 100000)

    vault.transfer(berg, swap_amount, {"from": deployer})
    assert vault._escrowedVaultTokens() == 0

    tx = vault.sendLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        swap_amount,
        [0, 0],
        berg,
        {"from": berg},
    )
    userBalance = vault.balanceOf(berg)
    assert vault._escrowedVaultTokens() == swap_amount

    txe = ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert vault.balanceOf(berg) == swap_amount + userBalance
    assert vault._escrowedVaultTokens() == 0


def test_only_one_response(vault, channel_id, ibc_emulator, berg, deployer):
    swap_percentage = 10000
    swap_amount = int(vault.totalSupply() * swap_percentage / 100000)

    vault.transfer(berg, swap_amount, {"from": deployer})

    tx = vault.sendLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
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


def test_ibc_ack_event(vault, channel_id, ibc_emulator, berg, deployer):
    """
    Test the SendLiquiditySuccess event gets fired.
    """
    swap_percentage = 10000
    swap_amount = int(vault.totalSupply() * swap_percentage / 100000)

    vault.transfer(berg, swap_amount, {"from": deployer})

    tx = vault.sendLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
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

    ack_event = txe.events["SendLiquiditySuccess"]

    assert ack_event["toAccount"].hex() == convert_64_bytes_address(berg.address).hex()
    assert ack_event["units"] == tx.return_value
    assert ack_event["escrowAmount"] == swap_amount
    assert ack_event["blockNumberMod"] == tx.block_number


def test_ibc_timeout_event(vault, channel_id, ibc_emulator, berg, deployer):
    """
    Test the SendLiquidityFailure event gets fired.
    """
    swap_percentage = 10000
    swap_amount = int(vault.totalSupply() * swap_percentage / 100000)

    vault.transfer(berg, swap_amount, {"from": deployer})

    tx = vault.sendLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
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

    timeout_event = txe.events["SendLiquidityFailure"]

    assert (
        timeout_event["toAccount"].hex() == convert_64_bytes_address(berg.address).hex()
    )
    assert timeout_event["units"] == tx.return_value
    assert timeout_event["escrowAmount"] == swap_amount
    assert timeout_event["blockNumberMod"] == tx.block_number
