import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain, convert


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


depositValues = [1000 * 10**18, 1000 * 10**18]


@pytest.fixture(scope="module")
def swappool1(deploy_swappool, accounts, token1, token2):
    yield deploy_swappool(
        [token1, token2],
        depositValues,
        2**62,
        "POOLNAME",
        "PS",
        weights=[1, 1],
        deployer=accounts[0],
    )


@pytest.fixture(scope="module")
def swappool2(deploy_swappool, accounts, token3):
    yield deploy_swappool(
        [token3],
        [1000 * 10**18],
        2**62,
        "POOLNAME",
        "PS",
        weights=[1, 1],
        deployer=accounts[0],
    )


def test_create_connections(swappool1, swappool2, accounts, chainId):
    swappool1.createConnection(
        chainId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        True,
        {"from": accounts[0]},
    )

    swappool2.createConnection(
        chainId,
        convert.to_bytes(swappool1.address.replace("0x", "")),
        True,
        {"from": accounts[0]},
    )

    swappool1.finishSetup({"from": accounts[0]})
    swappool2.finishSetup({"from": accounts[0]})


def test_deposit_into_pool(gov, accounts, swappool1, token1, token2):
    depositValue = 10**18 * 1000

    balance_modifier = accounts[2]

    token1.transfer(balance_modifier, depositValue, {"from": gov})
    token1.approve(swappool1, depositValue, {"from": balance_modifier})
    token2.transfer(balance_modifier, depositValue, {"from": gov})
    token2.approve(swappool1, depositValue, {"from": balance_modifier})

    baseAmount = (
        int((depositValue * swappool1.totalSupply()) / token1.balanceOf(swappool1))
        - 1000
    )
    swappool1.depositMixed([depositValue], baseAmount-1, {"from": balance_modifier})

    assert 10**5 > token1.balanceOf(balance_modifier)
    assert swappool1.balanceOf(balance_modifier) == baseAmount


def test_swap_one_direction(
    accounts, swappool1, swappool2, token1, chainId, ibcemulator
):
    swapValue = 10**18 * 100

    usr = accounts[0]
    token1.approve(swappool1, swapValue, {"from": usr})

    tx = swappool1.swapToUnits(
        chainId,
        convert.to_bytes(swappool2.address.replace("0x", "")),
        convert.to_bytes(usr.address.replace("0x", "")),
        token1,
        0,
        swapValue,
        0,
        0,  # Equal to False, False,
        usr,
        {"from": usr},
    )

    ibcemulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": usr},
    )

    ackTx = ibcemulator.ack(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": usr})
    assert swappool1._escrowedTokens(token1) == 0


def test_price_invariant(gov, accounts, swappool1, token1, token2):
    balance_modifier = accounts[2]

    chain.snapshot()

    amp = swappool1._amp() / 2**64
    priceBeforeWithdrawal = (
        (token1.balanceOf(swappool1) / token2.balanceOf(swappool1)) ** amp
        * swappool1._weight(token2)
        / swappool1._weight(token1)
    )
    topInvariantBeforeWithdrawal = token1.balanceOf(swappool1)**(1-amp) + token2.balanceOf(swappool1)**(1-amp)
    invariantBeforeWithdrawal = topInvariantBeforeWithdrawal

    chain.revert()

    baseAmount = swappool1.balanceOf(balance_modifier)
    assert baseAmount > 0
    txW = swappool1.withdrawAll(baseAmount, [0, 0, 0], {"from": balance_modifier})

    priceAfterWithdrawal = (
        (token1.balanceOf(swappool1) / token2.balanceOf(swappool1)) ** amp
        * swappool1._weight(token2)
        / swappool1._weight(token1)
    )
    topInvariantAfterWithdrawal = token1.balanceOf(swappool1)**(1-amp) + token2.balanceOf(swappool1)**(1-amp)
    invariantAfterWithdrawal = topInvariantAfterWithdrawal

    assert priceAfterWithdrawal == priceBeforeWithdrawal
    assert 1 + 1e-08 >= invariantAfterWithdrawal/invariantBeforeWithdrawal >= 1 - 1e-08
