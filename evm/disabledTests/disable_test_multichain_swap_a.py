import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, Token, chain
from brownie.test import given, strategy
from hypothesis import settings
import json, os


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# POOLNAME = "PS One Two Three"
# POOLSYMBOL = "ps(ott) "
POOLNAME = "PS OneTwoThree"
POOLSYMBOL = "ps(OTT) "


def test_name_length():
    assert len(POOLNAME) <= 16
    assert len(POOLSYMBOL) <= 8


@pytest.fixture(scope="module")
def create_swappool(gov, deploy_swappool, token1, token2, token3):
    def swappool(poolname, poolsymbol):
        tx = deploy_swappool([token1, token2, token3], 2**64, poolname, poolsymbol)
        sp = Token.at(tx.return_value)
        return sp

    yield swappool


def deposit(accounts, token1, token2, token3, sp, gov):

    depositValues = [10 * 10**18, 1000 * 10**18, 1000 * 10**6]

    base_account = accounts[1]
    token1.transfer(base_account, depositValues[0], {"from": gov})
    token2.transfer(base_account, depositValues[1], {"from": gov})
    token3.transfer(base_account, depositValues[2], {"from": gov})

    tokens = [token1, token2, token3]

    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositValues[i]

        token.approve(sp, depositValue, {"from": base_account})
        assert token.balanceOf(base_account) == depositValue
        assert token.allowance(base_account, sp) == depositValue

        sp.deposit(token, depositValue, {"from": base_account})

        pt = sp.poolToken(token)
        pt = Token.at(pt)
        assert pt.balanceOf(base_account) == depositValue
        assert token.balanceOf(base_account) == 0

    chain.snapshot()


# NOTE: Test IPC to be removed once Polymerase devnet scripts complete

# create some named fifos for test IPC
def create_fifos(write_pipe, read_pipe):

    if not os.path.exists(write_pipe):
        os.mkfifo(write_pipe)
    if not os.path.exists(read_pipe):
        os.mkfifo(read_pipe)

    read_handle = open(read_pipe, "r")
    write_handle = open(write_pipe, "w")

    return write_handle, read_handle


# write a json message to the other chain
def write_to(obj, f):
    print(json.dumps(obj), file=f)
    f.flush()


# read a json message from other chain
def read_from(f):
    return json.loads(f.readline())


@pytest.mark.no_call_coverage
# @given(swapValue=strategy("uint256", max_value=depositValues[0], min_value=10 ** 18))
def test_multipool_crosschain_swap(
    accounts,
    token1,
    token2,
    token3,
    gov,
    create_swappool,
    polymeraseemulator,
    crosschaininterface,
):

    swapValue = 10**18 * 5
    swappoolA = create_swappool("PS A", "PSA")

    wH, rH = create_fifos(
        "/tmp/test_multichain_swap_b.json", "/tmp/test_multichain_swap_a.json"
    )

    # make initial deposit to swappool
    deposit(accounts, token1, token2, token3, swappoolA, gov)

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappoolA, swapValue, {"from": base_account})

    # tell chain B about the swappool and chain id
    write_to(
        {
            "swappool": {"address": swappoolA.address},
            "ccsi": {"chain_id": crosschaininterface.chain_id()},
        },
        wH,
    )

    chainB_data = read_from(rH)
    ccsiB = chainB_data["ccsi"]
    swappoolB = chainB_data["swappool"]

    b = chainB_data["token2"]["balance"]
    x = swapValue
    a = token1.balanceOf(swappoolA)
    w = swappoolA.getBalance0(token1) / swappoolB["balance"]
    y = b * (1 - (a / (a + x)) ** w)

    B0 = a

    deviation = 0.02 / 100

    balanceZeros = [swappoolA.getBalance0(token1), swappoolB["balance"]]

    # swapToUnits
    # (_chain : uint256, _targetPool : bytes32, _fromAsset : address, _toAsset : uint256, _who : bytes32, _amount : uint256) -> uint256
    tokenArr = [
        swappoolA._tokenIndexing(0),
        swappoolA._tokenIndexing(1),
        swappoolA._tokenIndexing(2),
    ]
    token2AssetIndex = tokenArr.index(token2)
    assert swappoolA._tokenIndexing(token2AssetIndex) == token2

    # convert chain B addresses to bytes
    swappoolB_address_bytes = brownie.convert.to_bytes(
        swappoolB["address"].replace("0x", "")
    )
    ccib_address_bytes = brownie.convert.to_bytes(ccsiB["address"].replace("0x", ""))

    # setup chainB crosschaininterface contract as friendly (configurator as ZERO address)
    # Is this needed???
    # crosschaininterface.setfriendlyChainInterfaces(ccsiB['chain_id'], ZERO_ADDRESS, {"from": ZERO_ADDRESS})

    # connect pool A and B
    swappoolA.setConnection(
        ccsiB["chain_id"], swappoolB_address_bytes, True, {"from": gov}
    )

    # perform the cross chain swap (w/o Polymerase nothing will happen)
    tx = swappoolA.swapToUnits(
        ccsiB["chain_id"],
        swappoolB_address_bytes,
        brownie.convert.to_bytes(base_account.address.replace("0x", "")),
        token1,
        token2AssetIndex,
        swapValue,
        0,
        {"from": base_account},
    )

    # write the crosschaintx info to chainB, skipping Polymerase for now
    event = {}
    cross_chain_event = tx.events["CrossChainTxEvent"]
    for key, val in cross_chain_event.items():
        event[key] = str(val)
    write_to(event, wH)

    assert token1.balanceOf(base_account) == 0
    assert token2.balanceOf(base_account) == 0

    # read swappoolB data after swap
    chainB_data = read_from(rH)
    swappoolB = chainB_data["swappool"]

    U = tx.return_value / 2**64
    calcU = B0 * np.log((B0 + x) / B0)
    outAccordingToCalcU = b * (1 - np.exp(-calcU / b))

    assert (
        np.greater_equal(
            [swappoolA.getBalance0(token1), swappoolB["balance"]], balanceZeros
        ).sum()
        == 2
    )
    out = chainB_data["token2"]["balance"]

    assert token1.balanceOf(base_account) == 0

    if (swapValue < token1.balanceOf(swappoolA) / 1000) or (
        y == 0
    ):  # also covers swapValue == 0
        assert out <= y
    else:
        print(out / y)
        assert 1 + deviation >= out / y >= 1 - deviation * 100  # lower is 2%
        # The calculation should get more precise as swapValue goes up.

    # reset ...
    chain.revert()
