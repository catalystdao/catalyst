import pytest
from brownie import reverts, convert, web3
from brownie.test import given, strategy
from hypothesis import example
import re
from utils.common_utils import convert_64_bytes_address

from utils.vault_utils import compute_asset_swap_hash


pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_vaults")

# TODO add fees test (create fixture that sets up non-zero fees to the vault)


@pytest.mark.no_call_coverage
@example(swap_amount=2 * 10**15)
@example(swap_amount=100 * 10**18)
@given(swap_amount=strategy("uint256", max_value=2000 * 10**18))
def test_cross_vault_swap(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    vault_2_tokens,
    berg,
    deployer,
    ibc_emulator,
    compute_expected_swap,
    swap_amount,
):
    # TODO parametrize source_token and target_tokens?
    source_token = vault_1_tokens[0]
    target_token = vault_2_tokens[0]

    assert target_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault_1, swap_amount, {"from": berg})

    y = compute_expected_swap(swap_amount, source_token, target_token)["to_amount"]

    tx = vault_1.sendAsset(
        channel_id,
        convert_64_bytes_address(vault_2.address),
        convert_64_bytes_address(berg.address),
        source_token,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert source_token.balanceOf(berg) == 0

    # The swap may revert because of the security limit     #TODO mark these cases as 'skip'?
    if vault_2.getUnitCapacity() < tx.events["SendAsset"]["units"]:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg},
        )
        # Check that the transaction is fail on ack
        assert txe.events["Acknowledgement"]["acknowledgement"] == hex(1)
        # Ensure no tokens are transfered.
        assert "Transfer" not in txe.events.keys()
        return
    else:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg},
        )

    purchased_tokens = txe.events["ReceiveAsset"]["toAmount"]

    assert purchased_tokens == target_token.balanceOf(berg)

    # Make sure no more than it's theoretically due is given
    assert purchased_tokens <= y + 1 or purchased_tokens <= int(
        y * 1.000001
    ), "Swap returns more than theoretical"  # TODO set bound as test variable?

    # Make sure no much less than it's theoretically due is given (ignore check for very small swaps)
    if swap_amount > 1000:
        assert purchased_tokens >= y - 1 or purchased_tokens >= int(
            y * 9 / 10
        ), "Swap returns much less than theoretical"  # TODO set bound as test variable?


@pytest.mark.no_call_coverage
@example(swap_amount=2 * 10**15)
@example(swap_amount=8 * 10**18)
@given(swap_amount=strategy("uint256", max_value=10 * 10**18))
def test_cross_vault_swap_min_out(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    vault_2_tokens,
    berg,
    deployer,
    compute_expected_swap,
    ibc_emulator,
    swap_amount,
):

    source_token = vault_1_tokens[0]
    target_token = vault_2_tokens[0]

    assert target_token.balanceOf(berg) == 0

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault_1, swap_amount, {"from": berg})

    y = compute_expected_swap(swap_amount, source_token, target_token)["to_amount"]
    min_out = int(y * 1.2)  # Make sure the swap always fails

    tx = vault_1.sendAsset(
        channel_id,
        convert_64_bytes_address(vault_2.address),
        convert_64_bytes_address(berg.address),
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

    tx_e = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    assert tx_e.events["Acknowledgement"]["acknowledgement"] == hex(1)

    # Ensure no tokens are transfered.
    assert "Transfer" not in tx_e.events.keys()


def test_send_asset_event(
    channel_id, vault_1, vault_2, vault_1_tokens, berg, elwood, deployer
):
    """
    Test the SendAsset event gets fired.
    """

    swap_amount = 10**8
    min_out = 100

    source_token = vault_1_tokens[0]

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault_1, swap_amount, {"from": berg})

    tx = vault_1.sendAsset(
        channel_id,
        convert_64_bytes_address(vault_2.address),
        convert_64_bytes_address(
            elwood.address
        ),  # NOTE: not using the same account as the caller of the tx to make sure the 'toAccount' is correctly reported
        source_token,
        1,  # NOTE: use non-zero target asset index to make sure the field is set on the event (and not just left blank)
        swap_amount,
        min_out,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value

    send_asset_event = tx.events["SendAsset"]

    assert send_asset_event["channelId"].hex() == channel_id.hex()
    assert (
        send_asset_event["toVault"].hex()
        == convert_64_bytes_address(vault_2.address).hex()
    )
    assert (
        send_asset_event["toAccount"].hex()
        == convert_64_bytes_address(elwood.address).hex()
    )
    assert send_asset_event["fromAsset"] == source_token
    assert send_asset_event["toAssetIndex"] == 1
    assert send_asset_event["fromAmount"] == swap_amount
    assert send_asset_event["units"] == observed_units
    assert send_asset_event["minOut"] == min_out


def test_receive_swap_event(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    vault_2_tokens,
    berg,
    elwood,
    deployer,
    ibc_emulator,
):
    """
    Test the SendAsset event gets fired.
    """

    swap_amount = 10**8

    source_token = vault_1_tokens[0]
    target_token = vault_2_tokens[0]

    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault_1, swap_amount, {"from": berg})

    tx = vault_1.sendAsset(
        channel_id,
        convert_64_bytes_address(vault_2.address),
        convert_64_bytes_address(elwood.address),
        source_token,
        0,
        swap_amount,
        0,
        elwood,
        {"from": berg},
    )

    observed_units = tx.return_value

    txe = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    receive_swap_event = txe.events["ReceiveAsset"]

    assert receive_swap_event["channelId"].hex() == channel_id.hex()
    assert (
        receive_swap_event["fromVault"].hex()
        == convert_64_bytes_address(vault_1.address).hex()
    )
    assert receive_swap_event["toAccount"] == elwood.address
    assert receive_swap_event["toAsset"] == target_token
    assert receive_swap_event["units"] == observed_units
    assert receive_swap_event["toAmount"] == target_token.balanceOf(elwood)
