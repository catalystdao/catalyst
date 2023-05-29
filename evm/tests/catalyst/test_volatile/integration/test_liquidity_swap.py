import pytest
from brownie import convert, reverts
from brownie.test import given, strategy
from hypothesis import example
import re
from utils.common_utils import convert_64_bytes_address

pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_vaults")


@pytest.mark.no_call_coverage
@example(deposit_percentage=6000, swap_percentage=7000)
@given(
    deposit_percentage=strategy("uint256", max_value=20000),
    swap_percentage=strategy("uint256", max_value=10000),
)
def test_liquidity_swap(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    berg,
    deployer,
    ibc_emulator,
    compute_expected_liquidity_swap,
    swap_percentage,
    deposit_percentage,
):
    swap_percentage /= 10000
    deposit_percentage /= 10000

    deposit_amounts = [
        int(token.balanceOf(vault_1) * deposit_percentage) for token in vault_1_tokens
    ]
    [
        token.transfer(berg, amount, {"from": deployer})
        for amount, token in zip(deposit_amounts, vault_1_tokens)
    ]
    [
        token.approve(vault_1, amount, {"from": berg})
        for amount, token in zip(deposit_amounts, vault_1_tokens)
    ]

    estimatedVaultTokens = int(vault_1.totalSupply() * deposit_percentage)

    tx = vault_1.depositMixed(
        deposit_amounts, int(estimatedVaultTokens * 0.999), {"from": berg}
    )

    vault1_tokens = tx.return_value

    vault1_tokens_swapped = int(vault1_tokens * swap_percentage)

    computation = compute_expected_liquidity_swap(vault1_tokens_swapped)
    U, estimatedVault2Tokens = computation["units"], computation["to_amount"]

    tx = vault_1.sendLiquidity(
        channel_id,
        convert_64_bytes_address(vault_2.address),
        convert_64_bytes_address(berg.address),
        vault1_tokens_swapped,
        [int(estimatedVault2Tokens * 9 / 10), 0],
        berg,
        {"from": berg},
    )
    assert vault_1.balanceOf(berg) == vault1_tokens - vault1_tokens_swapped

    if vault_2.getUnitCapacity() < tx.events["SendLiquidity"]["units"]:
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

    purchased_tokens = txe.events["ReceiveLiquidity"]["toAmount"]

    assert purchased_tokens == vault_2.balanceOf(berg)

    assert purchased_tokens <= int(
        estimatedVault2Tokens * 1.000001
    ), "Swap returns more than theoretical"

    if swap_percentage < 1e-05:
        return

    assert (
        estimatedVault2Tokens * 9 / 10
    ) <= purchased_tokens, "Swap returns less than 9/10 theoretical"
