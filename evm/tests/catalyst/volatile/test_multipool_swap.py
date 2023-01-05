import brownie
import numpy as np
import pytest
from brownie.test import given, strategy
from hypothesis import settings
from a_common_functions import get_swap_return, check_swap_return


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# POOLNAME = "PS One Two Three"
# POOLSYMBOL = "ps(ott) "
POOLNAME = "PS OneTwoThree"
POOLSYMBOL = "ps(OTT) "


depositValues = [10 * 10**18, 1000 * 10**18, 1000 * 10**6]


@pytest.fixture(scope="module")
def create_swappool(accounts, deploy_swappool, token1, token2, token3):
    def swappool(poolname, poolsymbol, base_account=accounts[1]):
        sp = deploy_swappool(
            [token1, token2, token3],
            depositValues,
            2**64,
            poolname,
            poolsymbol,
            deployer=base_account,
        )

        return sp

    yield swappool


@pytest.fixture(scope="module")
def swappoolA(create_swappool, accounts):
    yield create_swappool("PS A", "PSA", base_account=accounts[2])


@pytest.fixture(scope="module")
def swappoolB(create_swappool, accounts):
    yield create_swappool("PS B", "PSB", base_account=accounts[2])


@pytest.mark.no_call_coverage
@given(swapValue=strategy("uint256", max_value=depositValues[0], min_value=10**18))
def test_multipool_swap(
    accounts,
    token1,
    token2,
    gov,
    swappoolA,
    swappoolB,
    ibcemulator,
    crosschaininterface,
    chainId,
    swapValue,
):

    base_account = accounts[2]

    # token1 to token2
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappoolA, swapValue, {"from": base_account})
    assert token2.balanceOf(base_account) == 0

    y = get_swap_return(swapValue, token1, token2, swappoolA, swappoolB)

    # swapToUnits
    # (_chain : uint256, _targetPool : bytes32, _fromAsset : address, _toAsset : uint256, _who : bytes32, _amount : uint256) -> uint256
    tokenArr = [
        swappoolA._tokenIndexing(0),
        swappoolA._tokenIndexing(1),
        swappoolA._tokenIndexing(2),
    ]
    token2AssetIndex = tokenArr.index(token2)
    assert swappoolA._tokenIndexing(token2AssetIndex) == token2

    # connect pool A and B
    swappoolA.createConnection(
        chainId,
        brownie.convert.to_bytes(swappoolB.address.replace("0x", "")),
        True,
        {"from": base_account},
    )
    swappoolB.createConnection(
        chainId,
        brownie.convert.to_bytes(swappoolA.address.replace("0x", "")),
        True,
        {"from": base_account},
    )

    tx = swappoolA.swapToUnits(
        chainId,
        brownie.convert.to_bytes(swappoolB.address.replace("0x", "")),
        brownie.convert.to_bytes(base_account.address.replace("0x", "")),
        token1,
        token2AssetIndex,
        swapValue,
        0,
        0,  # False, False
        base_account,
        {"from": base_account},
    )
    assert token1.balanceOf(base_account) == 0
    assert token2.balanceOf(base_account) == 0
    # Polymerase
    txe = ibcemulator.execute(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": base_account},
    )
    # swapFromUnits follows
    assert token2.balanceOf(base_account) > 0
    out = token2.balanceOf(base_account)

    assert token1.balanceOf(base_account) == 0

    check_swap_return(out, y, [swapValue, token1.balanceOf(swappoolA)])
