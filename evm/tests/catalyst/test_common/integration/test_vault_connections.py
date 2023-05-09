import pytest
from brownie import reverts, convert
import re

from utils.common_utils import convert_64_bytes_address
from tests.catalyst.utils.vault_utils import (
    encode_swap_payload,
    encode_liquidity_swap_payload,
)

pytestmark = [pytest.mark.usefixtures("group_finish_setup"), pytest.mark.no_vault_param]
# ! TODO when to test the following? before or after finish setup?


@pytest.fixture(scope="module")
def dummy_vault_address():
    # Provide a dummy target contract address. Does not need to be a valid vault as it is
    # not used to actually receive assets.
    yield convert.to_address("0xCA7A1AB5CA7A1AB5CA7A1AB5CA7A1AB5CA7A1AB5")


def test_connect_vaults(vault, deployer, channel_id, dummy_vault_address):

    # Make sure vaults are not connected
    assert not vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )

    vault.setConnection(
        channel_id,
        convert_64_bytes_address(dummy_vault_address),
        True,
        {"from": deployer},
    )

    # Make sure vaults are connected
    assert vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )


def test_disconnect_vaults(vault, deployer, channel_id, dummy_vault_address):

    vault.setConnection(
        channel_id,
        convert_64_bytes_address(dummy_vault_address),
        True,
        {"from": deployer},
    )

    # Make sure vaults are connected
    assert vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )

    vault.setConnection(
        channel_id,
        convert_64_bytes_address(dummy_vault_address),
        False,
        {"from": deployer},
    )

    # Make sure vaults are not connected
    assert not vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )


@pytest.mark.no_call_coverage
@pytest.mark.parametrize("connection_state", [True, False])
def test_create_connection_event(
    vault, deployer, connection_state, dummy_vault_address
):
    channel_id = convert.to_bytes(
        10, type_str="bytes32"
    )  # NOTE: using non-zero channel id to make sure event is correctly set

    tx = vault.setConnection(
        channel_id,
        convert_64_bytes_address(dummy_vault_address),
        connection_state,
        {"from": deployer},
    )

    event = tx.events["SetConnection"]

    assert event["channelId"] == convert.datatypes.HexString(
        channel_id, type_str="bytes32"
    )
    assert event["toVault"].hex() == convert_64_bytes_address(dummy_vault_address).hex()
    assert event["newState"] == connection_state


def test_connect_vaults_invalid_auth(vault, channel_id, elwood, dummy_vault_address):

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: No auth"
        vault.setConnection(
            channel_id,
            convert_64_bytes_address(dummy_vault_address),
            True,
            {"from": elwood},
        )


def test_not_connected_send_asset(
    channel_id, vault, vault_tokens, deployer, berg, dummy_vault_address
):

    # Make sure vaults are not connected
    assert not vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )

    source_token = vault_tokens[0]

    swap_amount = int(source_token.balanceOf(vault) * 0.1)

    source_token.approve(vault, swap_amount, {"from": deployer})

    with reverts(revert_pattern=re.compile("typed error: 0x86d01e67.*")):
        vault.sendAsset(
            channel_id,
            convert_64_bytes_address(dummy_vault_address),
            convert_64_bytes_address(berg.address),
            source_token,
            0,
            swap_amount,
            0,
            deployer,
            {"from": deployer},
        )


def test_not_connected_receive_swap(
    channel_id, vault, cross_chain_interface, ibc_emulator, berg, dummy_vault_address
):

    # Make sure vaults are not connected
    assert not vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )

    units = int(vault.getUnitCapacity() * 0.1)

    fake_payload = encode_swap_payload(
        dummy_vault_address,  # from vault
        vault.address,  # to vault
        berg.address,  # recipient
        units,
        0,
    )
    fake_packet = [["", channel_id], ["", ""], 0, fake_payload, [0, 0]]

    # with reverts(revert_pattern=re.compile("typed error: 0x86d01e67.*")):
    txe = cross_chain_interface.onRecvPacket(fake_packet, {"from": ibc_emulator})

    # Ensure no tokens are transfered.
    assert "Transfer" not in txe.events.keys()


def test_not_connected_out_liquidity(
    channel_id, vault, deployer, berg, dummy_vault_address
):

    # Make sure vaults are not connected
    assert not vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )

    swap_amount = int(vault.balanceOf(deployer) * 0.1)

    with reverts(revert_pattern=re.compile("typed error: 0x86d01e67.*")):
        vault.sendLiquidity(
            channel_id,
            convert_64_bytes_address(dummy_vault_address),  # to vault
            convert_64_bytes_address(berg.address),  # recipient
            swap_amount,
            [0, 0],
            deployer,
            {"from": deployer},
        )


def test_not_connected_in_liquidity(
    channel_id, vault, cross_chain_interface, ibc_emulator, berg, dummy_vault_address
):

    # Make sure vaults are not connected
    assert not vault._vaultConnection(
        channel_id, convert_64_bytes_address(dummy_vault_address)
    )

    units = int(vault.getUnitCapacity() * 0.1)

    fake_payload = encode_liquidity_swap_payload(
        dummy_vault_address,  # from vault
        vault.address,  # to vault
        berg.address,  # recipient
        units,
        [1, 0],
    )
    fake_packet = [["", channel_id], ["", ""], 0, fake_payload, [0, 0]]

    # with reverts(revert_pattern=re.compile("typed error: 0x86d01e67.*")):
    txe = cross_chain_interface.onRecvPacket(fake_packet, {"from": ibc_emulator})

    # Ensure no tokens are transfered.
    assert "Transfer" not in txe.events.keys()
