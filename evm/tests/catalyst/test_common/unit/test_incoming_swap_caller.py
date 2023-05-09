import pytest
from brownie import reverts, web3, convert, chain

from utils.common_utils import convert_64_bytes_address

pytestmark = [
    pytest.mark.usefixtures("vault_connect_itself"),
    pytest.mark.no_vault_param,
]


def test_receiveAsset_must_be_called_by_cci(
    channel_id,
    vault,
    berg,
):
    cci = vault._chainInterface()

    with reverts():
        vault.receiveAsset(
            channel_id,
            convert_64_bytes_address(vault.address),  # Use self as source vault
            0,
            berg,
            10**16,
            0,
            0,
            convert_64_bytes_address(vault.address),
            chain[-1].number,
            {"from": berg},
        )

    vault.receiveAsset(
        channel_id,
        convert_64_bytes_address(vault.address),  # Use self as source vault
        0,
        berg,
        10**16,
        0,
        0,
        convert_64_bytes_address(vault.address),
        chain[-1].number,
        {"from": cci},
    )


def test_receiveLiquidity_must_be_called_by_cci(
    channel_id,
    vault,
    berg,
):
    cci = vault._chainInterface()

    with reverts():
        vault.receiveLiquidity(
            channel_id,
            convert_64_bytes_address(vault.address),  # Use self as source vault
            berg,
            10**16,
            0,
            0,
            0,
            chain[-1].number,
            {"from": berg},
        )

    vault.receiveLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),  # Use self as source vault
        berg,
        10**16,
        0,
        0,
        0,
        chain[-1].number,
        {"from": cci},
    )


def test_release_escrow_must_be_called_cci(
    vault,
    berg,
):
    cci = vault._chainInterface()

    with reverts():  # "dev: Only _chainInterface"
        vault.onSendAssetSuccess(
            convert_64_bytes_address(berg.address),
            0,
            0,
            berg,
            convert.to_bytes(0),
            {"from": berg},
        )

    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts():  # "dev: Invalid swapHash. Alt: Escrow doesn't exist."
        vault.onSendAssetSuccess(
            convert_64_bytes_address(berg.address),
            0,
            0,
            berg,
            convert.to_bytes(0),
            {"from": cci},
        )


def test_timeout_escrow_must_be_called_cci(
    vault,
    berg,
):
    cci = vault._chainInterface()

    with reverts():  # "dev: Only _chainInterface"
        vault.onSendAssetFailure(
            convert_64_bytes_address(berg.address),
            0,
            0,
            berg,
            convert.to_bytes(0),
            {"from": berg},
        )

    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts():  # "dev: Invalid swapHash. Alt: Escrow doesn't exist."
        vault.onSendAssetFailure(
            convert_64_bytes_address(berg.address),
            0,
            0,
            berg,
            convert.to_bytes(0),
            {"from": cci},
        )


def test_release_liquidity_escrow_must_be_called_cci(
    vault,
    berg,
):
    cci = vault._chainInterface()

    with reverts():  # "dev: Only _chainInterface"
        vault.onSendLiquiditySuccess(
            convert_64_bytes_address(berg.address),
            0,
            0,
            convert.to_bytes(0),
            {"from": berg},
        )

    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts():  # "dev: Invalid swapHash. Alt: Escrow doesn't exist."
        vault.onSendLiquiditySuccess(
            convert_64_bytes_address(berg.address),
            0,
            0,
            convert.to_bytes(0),
            {"from": cci},
        )


def test_timeout_liquidity_escrow_must_be_called_cci(
    vault,
    berg,
):
    cci = vault._chainInterface()

    with reverts():  # "dev: Only _chainInterface"
        vault.onSendLiquidityFailure(
            convert_64_bytes_address(berg.address),
            0,
            0,
            convert.to_bytes(0),
            {"from": berg},
        )

    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts():  # "dev: Invalid swapHash. Alt: Escrow doesn't exist."
        vault.onSendLiquidityFailure(
            convert_64_bytes_address(berg.address),
            0,
            0,
            convert.to_bytes(0),
            {"from": cci},
        )
