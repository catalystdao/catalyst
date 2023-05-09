import pytest
from brownie import chain, convert, reverts
from brownie.test import given
from hypothesis import example
from hypothesis.strategies import floats
import re

from tests.catalyst.utils.vault_utils import compute_expected_units_capacity
from tests.catalyst.utils.common_utils import (
    assert_abs_relative_error,
    convert_64_bytes_address,
)


pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_vaults")

# TODO split into different tests?
@pytest.mark.no_call_coverage
@example(swap_amount_percentage=0.8)
@given(
    swap_amount_percentage=floats(min_value=0, max_value=2)
)  # From 0 to 2x the tokens hold by the vault
def test_security_limit_swap_loop(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    vault_2_tokens,
    berg,
    deployer,
    ibc_emulator,
    swap_amount_percentage,
):
    source_token = vault_1_tokens[0]
    target_token = vault_2_tokens[0]

    swap_amount = int(swap_amount_percentage * source_token.balanceOf(vault_1))

    assert source_token.balanceOf(berg) == 0

    # Cache max capacities for speed
    source_max_capacity = vault_1._maxUnitCapacity()
    target_max_capacity = vault_2._maxUnitCapacity()

    # Swap from vault_1 to vault_2
    # 1. sendAsset
    source_token.transfer(berg, swap_amount, {"from": deployer})
    source_token.approve(vault_1, swap_amount, {"from": berg})
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
    tx_units = tx.events["SendAsset"]["units"]

    # 2. receiveAsset
    if vault_2.getUnitCapacity() < tx_units:
        with reverts(revert_pattern=re.compile("typed error: 0x249c4e65.*")):
            txe = ibc_emulator.execute(
                tx.events["IncomingMetadata"]["metadata"][0],
                tx.events["IncomingPacket"]["packet"],
                {"from": berg},
            )
        return
    else:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg},
        )

    purchased_tokens = txe.events["ReceiveAsset"]["toAmount"]

    # 3. Ack
    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        txe.events["Acknowledgement"]["acknowledgement"],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    # Verify the security limit
    source_capacity = vault_1.getUnitCapacity()
    target_capacity = vault_2.getUnitCapacity()
    capacity_timestamp = chain[-1].timestamp

    assert (
        source_capacity == source_max_capacity
    )  # Will not change, since already at max capacity

    if (
        tx_units > 0
    ):  # Explicit check to make sure the capacity of the second vault has decreased (if units have been transferred)
        assert target_capacity < target_max_capacity

    # Verify target_capacity decrease calculation
    assert (
        abs(
            target_capacity
            - compute_expected_units_capacity(
                ref_capacity=target_max_capacity,
                ref_capacity_timestamp=chain[
                    0
                ].timestamp,  # Doesn't really matter, as the limit is already at max capacity
                change_timestamp=txe.timestamp,
                change_capacity_delta=-tx_units,
                current_timestamp=chain[-1].timestamp,
                max_capacity=target_max_capacity,
            )
        )
        <= 1
    )  # Allow 1 unit for rounding errors

    # Reverse-swap from vault_2 to vault_1
    # 4. sendAsset
    target_token.approve(vault_2, purchased_tokens, {"from": berg})
    tx2 = vault_2.sendAsset(
        channel_id,
        convert_64_bytes_address(vault_1.address),
        convert_64_bytes_address(berg.address),
        target_token,
        0,
        purchased_tokens,
        0,
        berg,
        {"from": berg},
    )
    tx2_units = tx2.events["SendAsset"]["units"]

    # Make sure the security limit of vault-2 does not change before the ack. NOTE: the unit capacity may have increased
    # slightly because of the time elapsed since the last capacity snapshot.

    # Intuitive check (unit capacity should not have changed much)
    assert_abs_relative_error(vault_2.getUnitCapacity(), target_capacity, 1e-2)
    # Accurate check
    assert (
        abs(
            vault_2.getUnitCapacity()
            - compute_expected_units_capacity(
                ref_capacity=target_capacity,
                ref_capacity_timestamp=capacity_timestamp,
                change_timestamp=tx2.timestamp,  # Doesn't really matter, as there has been no change
                change_capacity_delta=0,  # ! No change expected
                current_timestamp=chain[-1].timestamp,
                max_capacity=target_max_capacity,
            )
        )
        <= 1
    )  # Allow 1 unit for rounding errors

    # 5. receiveAsset
    txe2 = ibc_emulator.execute(
        tx2.events["IncomingMetadata"]["metadata"][0],
        tx2.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    # 6. Ack
    ibc_emulator.ack(
        tx2.events["IncomingMetadata"]["metadata"][0],
        txe2.events["Acknowledgement"]["acknowledgement"],
        tx2.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    # Verify security limit calculations
    assert (
        abs(
            vault_1.getUnitCapacity()
            - compute_expected_units_capacity(
                ref_capacity=source_capacity,
                ref_capacity_timestamp=capacity_timestamp,
                change_timestamp=txe2.timestamp,
                change_capacity_delta=-tx2_units,
                current_timestamp=chain[-1].timestamp,
                max_capacity=source_max_capacity,
            )
        )
        <= 1
    )  # Allow 1 unit for rounding errors

    # The security limit of vault_2 should have to return to (almost) max capacity
    # Intuitive check
    assert_abs_relative_error(
        vault_2.getUnitCapacity(), target_max_capacity, error_bound=1e-4
    )
    # Accurate check
    assert (
        abs(
            vault_2.getUnitCapacity()
            - compute_expected_units_capacity(
                ref_capacity=target_capacity,
                ref_capacity_timestamp=capacity_timestamp,
                change_timestamp=txe2.timestamp,
                change_capacity_delta=tx2_units,
                current_timestamp=chain[-1].timestamp,
                max_capacity=target_max_capacity,
            )
        )
        <= 1
    )  # Allow 1 unit for rounding errors


def test_security_limit_swap_timeout(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    vault_2_tokens,
    berg,
    deployer,
    ibc_emulator,
):
    source_token = vault_1_tokens[0]
    target_token = vault_2_tokens[0]

    # Send a quarter of vault_1 source_token balance
    swap_amount = int(source_token.balanceOf(vault_1) / 4)

    source_token.transfer(berg, swap_amount, {"from": deployer})

    # TODO create a fixture for this?
    # Swap from vault_1 to vault_2 to introduce a change in the security limit of vault_2
    # 1. sendAsset
    source_token.approve(vault_1, swap_amount, {"from": berg})
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

    # 2. receiveAsset
    txe = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg},
    )
    purchased_tokens = txe.events["ReceiveAsset"]["toAmount"]

    # 3. Ack
    ibc_emulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        txe.events["Acknowledgement"]["acknowledgement"],
        tx.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    # Cache the target security limit capacity before the swap
    target_capacity = vault_2.getUnitCapacity()
    capacity_timestamp = chain[-1].timestamp

    # Reverse-swap from vault_2 to vault_1
    # 4. sendAsset
    target_token.approve(vault_2, purchased_tokens, {"from": berg})
    tx2 = vault_2.sendAsset(
        channel_id,
        convert_64_bytes_address(vault_1.address),
        convert_64_bytes_address(berg.address),
        target_token,
        0,
        purchased_tokens,
        0,
        berg,
        {"from": berg},
    )

    # 6. Timeout
    ibc_emulator.timeout(
        tx2.events["IncomingMetadata"]["metadata"][0],
        tx2.events["IncomingPacket"]["packet"],
        {"from": deployer},
    )

    # Make sure the security limit of vault-1 does not change after the timeout (allow for the normal decay of the security limit)
    assert (
        abs(
            vault_2.getUnitCapacity()
            - compute_expected_units_capacity(
                ref_capacity=target_capacity,
                ref_capacity_timestamp=capacity_timestamp,
                change_timestamp=tx2.timestamp,
                change_capacity_delta=0,  # ! Expected capacity change set to 0
                current_timestamp=chain[-1].timestamp,
                max_capacity=vault_2._maxUnitCapacity(),
            )
        )
        <= 1
    )  # Allow 1 unit for rounding errors
