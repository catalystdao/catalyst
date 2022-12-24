import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, chain
from brownie.test import given, strategy
from hypothesis import settings


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# POOLNAME = "PS One Two Three"
# POOLSYMBOL = "ps(ott) "
POOLNAME = "PS OneTwoThree"
POOLSYMBOL = "ps(OTT) "


depositValues = [10 * 10**18, 1000 * 10**18, 1000 * 10**6]


@pytest.mark.no_call_coverage
@given(swapValue=strategy("uint256", max_value=depositValues[0] * 1))
def test_ibc_timeout(
    accounts,
    token1,
    token2,
    gov,
    ibcemulator,
    default_swappool_self,
    chainId,
    swapValue,
):
    swappool = default_swappool_self

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappool, swapValue, {"from": base_account})

    tokenArr = [
        swappool._tokenIndexing(0),
        swappool._tokenIndexing(1),
        swappool._tokenIndexing(2),
    ]
    toAssetIndex = tokenArr.index(token2)
    assert swappool._tokenIndexing(toAssetIndex) == token2

    tx = swappool.swapToUnits(
        chainId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(base_account.address.replace("0x", "")),
        token1,
        toAssetIndex,
        swapValue,
        0,
        0,
        base_account,
        {"from": base_account},
    )
    assert token1.balanceOf(base_account) == 0
    txe = ibcemulator.timeout(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": base_account},
    )
    assert token1.balanceOf(base_account) == swapValue


@pytest.mark.no_call_coverage
def test_ibc_ack(
    accounts,
    token1,
    token2,
    gov,
    ibcemulator,
    default_swappool_self,
    chainId,
    swapValue=10**18,
):
    swappool = default_swappool_self

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappool, swapValue, {"from": base_account})

    tokenArr = [
        swappool._tokenIndexing(0),
        swappool._tokenIndexing(1),
        swappool._tokenIndexing(2),
    ]
    toAssetIndex = tokenArr.index(token2)
    assert swappool._tokenIndexing(toAssetIndex) == token2

    tx = swappool.swapToUnits(
        chainId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(base_account.address.replace("0x", "")),
        token1,
        toAssetIndex,
        swapValue,
        0,
        0,
        base_account,
        {"from": base_account},
    )
    userBalance = token1.balanceOf(base_account)
    txe = ibcemulator.ack(
        tx.events["IncomingMetadata"]["metadata"][0],
        tx.events["IncomingPacket"]["packet"],
        {"from": base_account},
    )
    assert token1.balanceOf(base_account) == userBalance


@given(
    swapValue=strategy("uint256", max_value=depositValues[0] * 1, min_value=10**14)
)
def test_ibc_timeout_and_ack(
    accounts, token1, token2, gov, ibcemulator, default_swappool_self, chainId, swapValue
):
    swappool = default_swappool_self

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, 2 * swapValue, {"from": gov})
    token1.approve(swappool, 2 * swapValue, {"from": base_account})

    tokenArr = [
        swappool._tokenIndexing(0),
        swappool._tokenIndexing(1),
        swappool._tokenIndexing(2),
    ]
    toAssetIndex = tokenArr.index(token2)
    assert swappool._tokenIndexing(toAssetIndex) == token2

    U = int(693147180559945344 / 2)  # Example value used to test if the swap is corrected.

    both1_12 = swappool.dry_swap_both(token1, token2, 10**18, False)
    both1_21 = swappool.dry_swap_both(token2, token1, 10**18, False)
    to1 = swappool.dry_swap_to_unit(token1, 10**18, True)
    from1 = swappool.dry_swap_from_unit(token1, U, False)

    tx1 = swappool.swapToUnits(
        chainId,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(base_account.address.replace("0x", "")),
        token1,
        toAssetIndex,
        swapValue,
        0,
        1,  # Equal to True, False.
        base_account,
        {"from": base_account},
    )

    both2_12 = swappool.dry_swap_both(token1, token2, 10**18, False)
    both2_21 = swappool.dry_swap_both(token2, token1, 10**18, False)
    to2 = swappool.dry_swap_to_unit(token1, 10**18, True)
    from2 = swappool.dry_swap_from_unit(token1, U, False)

    assert both1_12 > both2_12
    assert both1_21 == both2_21
    assert to1 > to2
    assert from1 == from2

    chain.snapshot()

    txe = ibcemulator.timeout(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": base_account},
    )

    both3_12 = swappool.dry_swap_both(token1, token2, 10**18, False)
    both3_21 = swappool.dry_swap_both(token2, token1, 10**18, False)
    to3 = swappool.dry_swap_to_unit(token1, 10**18, True)
    from3 = swappool.dry_swap_from_unit(token1, U, False)

    assert both1_12 == both3_12
    assert both1_21 == both3_21
    assert to1 == to3
    assert from1 == from3

    chain.revert()

    txe = ibcemulator.ack(
        tx1.events["IncomingMetadata"]["metadata"][0],
        tx1.events["IncomingPacket"]["packet"],
        {"from": base_account},
    )

    both3_12 = swappool.dry_swap_both(token1, token2, 10**18, False)
    both3_21 = swappool.dry_swap_both(token2, token1, 10**18, False)
    to3 = swappool.dry_swap_to_unit(token1, 10**18, True)
    from3 = swappool.dry_swap_from_unit(token1, U, False)

    assert both1_12 > both3_12
    assert both1_21 < both3_21
    assert to1 > to3
    assert from1 < from3
