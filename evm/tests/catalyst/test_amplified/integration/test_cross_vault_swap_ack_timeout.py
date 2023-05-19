import pytest
from brownie import reverts, convert, web3, chain
from brownie.test import given, strategy
from hypothesis.strategies import floats
from hypothesis import example
from utils.common_utils import convert_64_bytes_address

from utils.vault_utils import compute_asset_swap_hash


pytestmark = [
    pytest.mark.usefixtures("vault_connect_itself"),
    pytest.mark.no_vault_param,
]

# TODO do we want to parametrize the swap_amount? (as it is right now)
@pytest.mark.no_call_coverage
@example(swap_amount=0.12)
@given(
    swap_amount=floats(min_value=0, max_value=0.5)
)  # From 0 to 2x the tokens hold by the vault
def test_ibc_ack(
    channel_id, vault, vault_tokens, ibc_emulator, berg, deployer, swap_amount
):

    source_token = vault_tokens[0]
    swap_amount = int(swap_amount * source_token.balanceOf(vault))

    assert source_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault, swap_amount, {"from": berg})

    tx = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        source_token,
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert source_token.balanceOf(berg) == 0

    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert source_token.balanceOf(berg) == 0


# TODO do we want to parametrize the swap_amount? (as it is right now)
@pytest.mark.no_call_coverage
@example(swap_amount=0.12)
@given(
    swap_amount=floats(min_value=0, max_value=0.5)
)  # From 0 to 2x the tokens hold by the vault
def test_ibc_timeout(
    channel_id, vault, vault_tokens, ibc_emulator, berg, deployer, swap_amount
):

    source_token = vault_tokens[0]
    swap_amount = int(swap_amount * source_token.balanceOf(vault))

    assert source_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault, swap_amount, {"from": berg})

    tx = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        source_token,
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert source_token.balanceOf(berg) == 0

    ibc_emulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )
    assert source_token.balanceOf(berg) == swap_amount  # Swapper gets fully refunded


def test_only_one_response(
    channel_id, vault, vault_tokens, ibc_emulator, berg, deployer
):

    source_token = vault_tokens[0]
    swap_amount = int(0.25 * source_token.balanceOf(vault))

    assert source_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault, swap_amount, {"from": berg})

    tx = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        source_token,
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
        ibc_emulator.ack(  # Same as timeout
            tx.events["IncomingMetadata"]["metadata"][0],
            convert.to_bytes(1, "bytes"),
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

    chain.undo(4)

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
        ibc_emulator.ack(  # Same as timeout
            tx.events["IncomingMetadata"]["metadata"][0],
            convert.to_bytes(1, "bytes"),
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


@example(swap_amount=2)
@given(
    swap_amount=floats(min_value=0.00001, max_value=5)
)  # From 0 to 5x the tokens hold by the vault
def test_ibc_timeout_and_ack(
    channel_id, vault, vault_tokens, ibc_emulator, berg, deployer, swap_amount
):

    if len(vault_tokens) < 2:
        pytest.skip("Need at least 2 tokens within a vault to run a local swap.")

    source_token = vault_tokens[0]
    target_token = vault_tokens[1]

    swap_amount = int(swap_amount * source_token.balanceOf(vault))

    assert source_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault, swap_amount, {"from": berg})

    U = 0
    for token in vault_tokens:
        U += (vault._weight(token) * token.balanceOf(vault)) ** (
            (10**18 - (10**18 - vault._oneMinusAmp())) / 10**18
        ) * 1000000

    both1_12 = vault.calcLocalSwap(source_token, target_token, 10**18)
    both1_21 = vault.calcLocalSwap(target_token, source_token, 10**18)
    to1 = vault.calcSendAsset(source_token, 10**18)
    from1 = vault.calcReceiveAsset(source_token, U)

    tx1 = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        source_token,
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )

    both2_12 = vault.calcLocalSwap(source_token, target_token, 10**18)
    both2_21 = vault.calcLocalSwap(target_token, source_token, 10**18)
    to2 = vault.calcSendAsset(source_token, 10**18)
    from2 = vault.calcReceiveAsset(source_token, U)

    assert both1_12 > both2_12
    assert both1_21 == both2_21
    assert to1 > to2
    assert from1 == from2

    txe = ibc_emulator.timeout(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = vault.calcLocalSwap(source_token, target_token, 10**18)
    both3_21 = vault.calcLocalSwap(target_token, source_token, 10**18)
    to3 = vault.calcSendAsset(source_token, 10**18)
    from3 = vault.calcReceiveAsset(source_token, U)

    assert both1_12 == both3_12
    assert both1_21 == both3_21
    assert to1 == to3
    assert from1 == from3

    chain.undo()

    txe = ibc_emulator.ack(  # Same as timeout
        tx1.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(1, "bytes"),
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = vault.calcLocalSwap(source_token, target_token, 10**18)
    both3_21 = vault.calcLocalSwap(target_token, source_token, 10**18)
    to3 = vault.calcSendAsset(source_token, 10**18)
    from3 = vault.calcReceiveAsset(source_token, U)

    assert both1_12 == both3_12
    assert both1_21 == both3_21
    assert to1 == to3
    assert from1 == from3

    chain.undo()

    txe = ibc_emulator.ack(
        tx1.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
        tx1.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    both3_12 = vault.calcLocalSwap(source_token, target_token, 10**18)
    both3_21 = vault.calcLocalSwap(target_token, source_token, 10**18)
    to3 = vault.calcSendAsset(source_token, 10**18)
    from3 = vault.calcReceiveAsset(source_token, U)

    assert both1_12 > both3_12
    assert both1_21 < both3_21
    assert to1 > to3
    assert from1 < from3


def test_ibc_ack_event(channel_id, vault, vault_tokens, ibc_emulator, berg, deployer):
    """
    Test the SendAssetSuccess event gets fired.
    """

    swap_amount = 10**8

    source_token = vault_tokens[0]

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault, swap_amount, {"from": berg})

    tx = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        source_token,
        1,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )

    txe = ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        convert.to_bytes(0, "bytes"),
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    ack_event = txe.events["SendAssetSuccess"]

    assert ack_event["toAccount"].hex() == convert_64_bytes_address(berg.address).hex()
    assert ack_event["U"] == tx.return_value
    assert ack_event["escrowAmount"] == swap_amount
    assert ack_event["escrowToken"] == source_token.address
    assert ack_event["blockNumberMod"] == tx.block_number


def test_ibc_timeout_event(
    channel_id, vault, vault_tokens, ibc_emulator, berg, deployer
):
    """
    Test the SendAssetFailure event gets fired.
    """

    swap_amount = 10**8

    source_token = vault_tokens[0]

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault, swap_amount, {"from": berg})

    tx = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        source_token,
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

    timeout_event = txe.events["SendAssetFailure"]

    assert (
        timeout_event["toAccount"].hex() == convert_64_bytes_address(berg.address).hex()
    )
    assert timeout_event["U"] == tx.return_value
    assert timeout_event["escrowAmount"] == swap_amount
    assert timeout_event["escrowToken"] == source_token.address
    assert timeout_event["blockNumberMod"] == tx.block_number
