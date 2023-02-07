import pytest
from brownie import reverts, convert

from tests.catalyst.utils.pool_utils import encode_swap_payload, encode_liquidity_swap_payload

pytestmark = pytest.mark.usefixtures("group_finish_setup")
# ! TODO when to test the following? before or after finish setup?


@pytest.fixture(scope="module")
def dummy_pool_address():
    # Provide a dummy target contract address. Does not need to be a valid pool as it is
    # not used to actually receive assets.
    yield convert.to_address('0xCA7A1AB5CA7A1AB5CA7A1AB5CA7A1AB5CA7A1AB5')


def test_connect_pools(
    pool,
    deployer,
    cross_chain_interface,
    channel_id,
    dummy_pool_address
):

    # Make sure pools are not connected
    assert not cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)

    pool.setConnection(
        channel_id,
        dummy_pool_address,
        True,
        {"from": deployer}
    )
    
    # Make sure pools are connected
    assert cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)



def test_disconnect_pools(
    pool,
    deployer,
    cross_chain_interface,
    channel_id,
    dummy_pool_address
):

    pool.setConnection(
        channel_id,
        dummy_pool_address,
        True,
        {"from": deployer}
    )
    
    # Make sure pools are connected
    assert cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)

    pool.setConnection(
        channel_id,
        dummy_pool_address,
        False,
        {"from": deployer}
    )
    
    # Make sure pools are not connected
    assert not cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)



@pytest.mark.no_call_coverage
@pytest.mark.parametrize("connection_state", [True, False])
def test_create_connection_event(
    pool,
    deployer,
    connection_state,
    dummy_pool_address
):
    channel_id = convert.to_bytes(10, type_str="bytes32")     # NOTE: using non-zero channel id to make sure event is correctly set

    tx = pool.setConnection(
        channel_id,
        dummy_pool_address,
        connection_state,
        {"from": deployer}
    )

    event = tx.events["SetConnection"]

    assert event["channelId"]  == convert.datatypes.HexString(channel_id, type_str="bytes32")
    assert event["targetPool"] == dummy_pool_address
    assert event["newState"]   == connection_state



def test_connect_pools_invalid_auth(
    pool,
    channel_id,
    elwood,
    dummy_pool_address
):

    with reverts(dev_revert_msg="dev: No auth"):
        pool.setConnection(
            channel_id,
            dummy_pool_address,
            True,
            {"from": elwood}
        )



def test_not_connected_send_swap(
    channel_id,
    pool,
    pool_tokens,
    cross_chain_interface,
    deployer,
    berg,
    dummy_pool_address
):

    # Make sure pools are not connected
    assert not cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)

    source_token = pool_tokens[0]

    swap_amount = int(source_token.balanceOf(pool) * 0.1)

    source_token.approve(pool, swap_amount, {"from": deployer})

    with reverts("No Connection"):
        pool.sendSwap(
            channel_id,
            convert.to_bytes(dummy_pool_address),
            convert.to_bytes(berg.address),
            source_token,
            0,
            swap_amount,
            0,
            deployer,
            {"from": deployer},
        )



def test_not_connected_receive_swap(
    channel_id,
    pool,
    cross_chain_interface,
    ibc_emulator,
    berg,
    dummy_pool_address
):

    # Make sure pools are not connected
    assert not cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)

    units = int(pool.getUnitCapacity() * 0.1)

    fake_payload = encode_swap_payload(
        dummy_pool_address,                         # from pool
        pool.address,                               # to pool
        convert.to_bytes(berg.address),             # recipient
        units,
        0
    )
    fake_packet = [["", channel_id], ["", ""], 0, fake_payload, [0, 0]]

    with reverts("No Connection"):
        cross_chain_interface.onRecvPacket(fake_packet, {"from": ibc_emulator})



def test_not_connected_out_liquidity(
    channel_id,
    pool,
    cross_chain_interface,
    deployer,
    berg,
    dummy_pool_address
):

    # Make sure pools are not connected
    assert not cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)

    swap_amount = int(pool.balanceOf(deployer) * 0.1)

    with reverts("No Connection"):
        pool.outLiquidity(
            channel_id,
            convert.to_bytes(dummy_pool_address),   # to pool
            convert.to_bytes(berg.address),         # recipient
            swap_amount,
            0,
            deployer,
            {"from": deployer}
        )



def test_not_connected_in_liquidity(
    channel_id,
    pool,
    cross_chain_interface,
    ibc_emulator,
    berg,
    dummy_pool_address
):

    # Make sure pools are not connected
    assert not cross_chain_interface.checkConnection(channel_id, pool.address, dummy_pool_address)

    units = int(pool.getUnitCapacity() * 0.1)

    fake_payload = encode_liquidity_swap_payload(
        dummy_pool_address,                         # from pool
        pool.address,                               # to pool
        convert.to_bytes(berg.address),             # recipient
        units,
        1,
    )
    fake_packet = [["", channel_id], ["", ""], 0, fake_payload, [0, 0]]

    with reverts("No Connection"):
        cross_chain_interface.onRecvPacket(fake_packet, {"from": ibc_emulator})