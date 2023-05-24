from constants import MSG_SENDER, ADDRESS_THIS, BALANCE_THIS
import pytest
from brownie import convert, reverts
import re


def byte_sum(elements: list):
    b = b""
    for elem in elements:
        b += elem
    return b


@pytest.fixture(scope="module")
def token1(create_token):
    yield create_token("Token1", "TKN1", 18, 10000)


@pytest.fixture(scope="module")
def vault1_tkns(weth, token1):
    yield [weth, token1]


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


def test_deposit_mixed(
    catalyst_router, vault11, weth, token1, accounts, berg, encode_router_payload
):

    amount = 10**18

    # Actions: Wrap ETH, deposited mixed (only ETH) then sweep.
    payload1 = encode_router_payload(
        [0x08, 0x0C, 0x04],
        [
            [ADDRESS_THIS, amount],
            [vault11.address, [weth.address, token1.address], [BALANCE_THIS, 0], 0],
            [vault11.address, MSG_SENDER, 0],
        ],
    )

    tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})

    assert vault11.balanceOf(berg) == 499875062460964


def test_deposit_then_withdraw_mixed(
    catalyst_router, vault11, weth, token1, accounts, berg, encode_router_payload
):

    amount = 10**18

    # Actions: Wrap ETH, deposited mixed (only ETH) then sweep.
    payload1 = encode_router_payload(
        [0x08, 0x0C, 0x0B, 0x04, 0x04],
        [
            [ADDRESS_THIS, amount],
            [
                vault11.address,
                [weth.address, token1.address],
                [BALANCE_THIS, 0],
                0,
            ],  # Should mint 499875062460964 tokens.
            [vault11.address, BALANCE_THIS, [10**17, 10**18, 0], [0, 0, 0]],
            [weth.address, MSG_SENDER, 0],
            [token1.address, MSG_SENDER, 0],
        ],
    )

    tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})

    assert weth.balanceOf(berg) == 100044983508657187
    assert token1.balanceOf(berg) == 899145825694949000


def test_deposit_then_withdraw_mixed_minout_fail(
    catalyst_router, vault11, weth, token1, berg, encode_router_payload
):

    amount = 10**18

    # Actions: Wrap ETH, deposited mixed (only ETH) then sweep.
    payload1 = encode_router_payload(
        [0x08, 0x0C, 0x0B, 0x04, 0x04],
        [
            [ADDRESS_THIS, amount],
            [
                vault11.address,
                [weth.address, token1.address],
                [BALANCE_THIS, 0],
                0,
            ],  # Should mint 499875062460964 tokens.
            [
                vault11.address,
                BALANCE_THIS,
                [10**17, 10**18, 0],
                [100044983508657187 + 1, 0, 0],
            ],
            [weth.address, MSG_SENDER, 0],  # Should return 100044983508657187
            [token1.address, MSG_SENDER, 0],  # Should return 899145825694949000
        ],
    )

    with reverts(revert_pattern=re.compile("typed error: 0x24557f05.*")):
        tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})


def test_deposit_then_withdraw_equal(
    catalyst_router, vault11, weth, token1, berg, encode_router_payload
):

    amount = 10**18

    # Actions: Wrap ETH, deposited mixed (only ETH) then sweep.
    payload1 = encode_router_payload(
        [0x08, 0x0C, 0x0A, 0x04, 0x04],
        [
            [ADDRESS_THIS, amount],
            [
                vault11.address,
                [weth.address, token1.address],
                [BALANCE_THIS, 0],
                0,
            ],  # Should mint 499875062460964 tokens.
            [vault11.address, BALANCE_THIS, [0, 0, 0]],
            [weth.address, MSG_SENDER, 0],
            [token1.address, MSG_SENDER, 0],
        ],
    )

    tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})

    assert weth.balanceOf(berg) == 500124937539034353
    assert token1.balanceOf(berg) == 499625312226807545


def test_deposit_then_withdraw_equal_minout_on_sweep(
    catalyst_router, vault11, weth, token1, berg, encode_router_payload
):

    amount = 10**18

    # Actions: Wrap ETH, deposited mixed (only ETH) then sweep.
    payload1 = encode_router_payload(
        [0x08, 0x0C, 0x0A, 0x04, 0x04],
        [
            [ADDRESS_THIS, amount],
            [
                vault11.address,
                [weth.address, token1.address],
                [BALANCE_THIS, 0],
                0,
            ],  # Should mint 499875062460964 tokens.
            [vault11.address, BALANCE_THIS, [0, 0, 0]],
            [weth.address, MSG_SENDER, 0],
            [
                token1.address,
                MSG_SENDER,
                499625312226807545 + 1,
            ],  # Should return 499625312226807545
        ],
    )

    with reverts("typed error: 0x675cae38"):
        tx = catalyst_router.execute(*payload1, {"from": berg.address, "value": amount})
