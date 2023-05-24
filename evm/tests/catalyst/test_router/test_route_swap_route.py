from constants import MSG_SENDER, ADDRESS_THIS, BALANCE_THIS
import pytest
from brownie import convert
from utils.common_utils import convert_64_bytes_address


def byte_sum(elements: list):
    b = b""
    for elem in elements:
        b += elem
    return b


@pytest.fixture(scope="module")
def token1(create_token):
    yield create_token("Token1", "TKN1", 18, 10000)


@pytest.fixture(scope="module")
def token2(create_token):
    yield create_token("Token2", "TKN2", 18, 10000)


@pytest.fixture(scope="module")
def token3(create_token):
    yield create_token("Token3", "TKN3", 18, 10000)


@pytest.fixture(scope="module")
def vault1_tkns(weth, token1):
    yield [weth, token1]


@pytest.fixture(scope="module")
def vault2_tkns(token2, token3):
    yield [token2, token3]


@pytest.fixture(scope="module")
def vault11(deployvault, vault1_tkns, accounts, weth):

    weth.deposit({"from": accounts[0], "value": 1000 * 10**18})

    yield deployvault(
        tokens=vault1_tkns,
        token_balances=[1000 * 10**18, 1000 * 10**18],
        weights=[1, 1],
        amp=10**18,
        name="Vault 11",
        symbol="P11",
    )


@pytest.fixture(scope="module")
def vault12(deployvault, token1):
    yield deployvault(
        tokens=[token1],
        token_balances=[1000 * 10**18],
        weights=[1],
        amp=10**18,
        name="Vault 12",
        symbol="P12",
    )


@pytest.fixture(scope="module")
def vault21(deployvault, token2):
    yield deployvault(
        tokens=[token2],
        token_balances=[1000 * 10**18],
        weights=[1],
        amp=10**18,
        name="Vault 21",
        symbol="P21",
    )


@pytest.fixture(scope="module")
def vault22(deployvault, vault2_tkns):
    yield deployvault(
        tokens=vault2_tkns,
        token_balances=[1000 * 10**18, 1000 * 10**18],
        weights=[1, 1],
        amp=10**18,
        name="Vault 21",
        symbol="P21",
    )


def test_sendSwap(
    catalyst_router,
    vault11,
    vault12,
    vault21,
    vault22,
    weth,
    token1,
    token2,
    token3,
    channel_id,
    ibc_emulator,
    accounts,
    berg,
    encode_router_payload,
):

    vault12.setConnection(
        channel_id,
        convert_64_bytes_address(vault21.address),
        True,
        {"from": accounts[0]},
    )
    vault21.setConnection(
        channel_id,
        convert_64_bytes_address(vault12.address),
        True,
        {"from": accounts[0]},
    )

    amount = 10**18
    payload1 = encode_router_payload(
        [0x08, 0x00, 0x01],
        [
            [ADDRESS_THIS, amount],
            [vault11.address, weth.address, token1.address, amount, 0],
            [
                vault12.address,
                channel_id,
                convert_64_bytes_address(vault21.address),
                convert_64_bytes_address(berg.address),
                token1.address,
                0,
                BALANCE_THIS,
                0,
                berg.address,
            ],  # Leave the calldata param blank.
        ],
    )

    tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})

    txe = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    assert token2.balanceOf(berg) > 998000000000000000


def test_route_route(
    catalyst_router,
    vault11,
    vault12,
    vault21,
    vault22,
    weth,
    token1,
    token2,
    token3,
    channel_id,
    ibc_emulator,
    accounts,
    berg,
    encode_router_payload,
):

    vault12.setConnection(
        channel_id,
        convert_64_bytes_address(vault21.address),
        True,
        {"from": accounts[0]},
    )
    vault21.setConnection(
        channel_id,
        convert_64_bytes_address(vault12.address),
        True,
        {"from": accounts[0]},
    )

    amount = 10**18

    payload2 = encode_router_payload(
        [0x00, 0x04],
        [
            [vault22.address, token2.address, token3.address, BALANCE_THIS, 0],
            [token3.address, berg.address, 0],
        ],
    )

    payloadTarget = convert.to_bytes(
        catalyst_router.address, type_str="bytes20"
    ) + convert.to_bytes(catalyst_router.execute.encode_input(*payload2)[10:], "bytes")

    payload1 = encode_router_payload(
        [0x08, 0x00, 0x01],
        [
            [ADDRESS_THIS, amount],
            [vault11.address, weth.address, token1.address, amount, 0],
            [
                vault12.address,
                channel_id,
                convert_64_bytes_address(vault21.address),
                convert_64_bytes_address(catalyst_router.address),
                token1.address,
                0,
                BALANCE_THIS,
                0,
                berg.address,
                payloadTarget,
            ],
        ],
    )

    tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})

    txe = ibc_emulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": berg},
    )

    assert token3.balanceOf(berg) > 997000000000000000
