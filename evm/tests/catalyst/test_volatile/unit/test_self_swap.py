import pytest
from brownie import reverts, convert
from brownie.test import given, strategy
from hypothesis import example
from hypothesis.strategies import floats
import re
from utils.common_utils import convert_64_bytes_address

pytestmark = pytest.mark.usefixtures("vault_connect_itself")


@pytest.mark.no_call_coverage
@example(swap_percentage=0.8)
@given(
    swap_percentage=floats(min_value=0, max_value=2)
)  # From 0 to 2x the tokens hold by the vault
def test_self_swap(
    vault, vault_tokens, berg, deployer, channel_id, ibc_emulator, swap_percentage
):
    token = vault_tokens[0]
    swap_amount = int(swap_percentage * token.balanceOf(vault))

    assert token.balanceOf(berg) == 0

    token.transfer(berg, swap_amount, {"from": deployer})
    token.approve(vault, swap_amount, {"from": berg})

    tx = vault.sendAsset(
        channel_id,
        convert_64_bytes_address(vault.address),
        convert_64_bytes_address(berg.address),
        token,
        0,
        swap_amount,
        0,
        berg,
        {"from": berg},
    )
    assert token.balanceOf(berg) == 0

    if vault.getUnitCapacity() < tx.events["SendAsset"]["Units"]:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg},
        )
        assert txe.events["Acknowledgement"]["acknowledgement"].hex() == "01"

        return
    else:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg},
        )

    purchased_tokens = txe.events["ReceiveAsset"]["toAmount"]

    assert token.balanceOf(berg) == purchased_tokens

    # We don't check that the swap returns less than a certain threshold because the escrow functionality impacts how close the swap can actually get to 1:1. Also, it should always return less than the input. INcluding errors.
    assert purchased_tokens <= swap_amount, "Swap returns more than theoretical"
