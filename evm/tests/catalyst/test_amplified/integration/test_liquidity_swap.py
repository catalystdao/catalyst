import pytest
from brownie import convert, reverts
from brownie.test import given, strategy
from hypothesis import example
import tests.catalyst.utils.vault_utils as vault_utils
import re
from utils.common_utils import convert_64_bytes_address

pytestmark = pytest.mark.usefixtures("group_finish_setup", "group_connect_vaults")


@pytest.mark.no_call_coverage
@example(deposit_percentage=4000, swap_percentage=6000)
@given(
    deposit_percentage=strategy("uint256", max_value=20000),
    swap_percentage=strategy("uint256", max_value=10000),
)
def test_liquidity_swap(
    channel_id,
    vault_1,
    vault_2,
    vault_1_tokens,
    vault_2_tokens,
    get_vault_2_weights,
    get_vault_2_balances,
    get_vault_2_unit_tracker,
    get_vault_2_amp,
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
    U, estimatedVault2Tokens = computation["Units"], computation["to_amount"]

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

    b0_times_n = len(vault_2_tokens) * vault_utils.compute_balance_0(
        get_vault_2_weights(),
        get_vault_2_balances(),
        get_vault_2_unit_tracker(),
        get_vault_2_amp(),
    )

    U = tx.events["SendLiquidity"]["Units"]
    expectedB0 = 2**256
    if int(int(b0_times_n) ** (1 - get_vault_2_amp() / 10**18)) >= int(U / 10**18):
        expectedB0 = vault_utils.compute_expected_swap_given_U(
            U, 1, b0_times_n, get_vault_2_amp()
        )

    securityLimit = vault_2.getUnitCapacity()
    if securityLimit < expectedB0:
        txe = ibc_emulator.execute(
            tx.events["IncomingMetadata"]["metadata"][0],
            tx.events["IncomingPacket"]["packet"],
            {"from": berg},
        )

        # If the transaction still executed, it needs to have exhausted the vast majority of the security limit.
        if txe.events["Acknowledgement"]["acknowledgement"].hex() == "00":
            assert (
                vault_2.getUnitCapacity() / securityLimit <= 0.015
            ), "Either test incorrect or security limit is not strict enough."

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
