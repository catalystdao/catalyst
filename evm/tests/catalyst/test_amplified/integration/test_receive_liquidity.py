import pytest
from utils.common_utils import convert_64_bytes_address

pytestmark = pytest.mark.usefixtures("vault_connect_itself")


def test_receive_liquidity(
    channel_id,
    vault,
    vault_tokens,
    berg,
    cross_chain_interface,
):

    units = 100000000000000
    # units = 200000000000000   # not working

    tx = vault.receiveLiquidity(
        channel_id,
        convert_64_bytes_address(vault.address),
        berg.address,
        units,
        0,
        0,
        0,
        0,
        {"from": cross_chain_interface},
    )

    purchased_tokens = tx.events["ReceiveLiquidity"]["toAmount"]

    print("Total supply:    ", vault.totalSupply())
    print("Received tokens: ", purchased_tokens)

