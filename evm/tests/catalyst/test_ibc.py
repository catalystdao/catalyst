import brownie
import numpy as np
import pytest
from brownie import ZERO_ADDRESS, Token, SwapPool, chain
from brownie.test import given, strategy
from hypothesis import settings
from a_common_functions import get_swap_return, check_swap_return, return_swap_check


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


# POOLNAME = "PS One Two Three"
# POOLSYMBOL = "ps(ott) "
POOLNAME = "PS OneTwoThree"
POOLSYMBOL = "ps(OTT) "

depositValues = [10 * 10**18, 1000 * 10**18, 1000 * 10**6]


@pytest.mark.no_call_coverage
# @given(swapValue=strategy("uint256", max_value=depositValues[0]/3, min_value=10**18))
def test_ibc_swap(
    accounts, token1, token2, gov, default_swappool_self, ibcemulator, crosschaininterface, chainId, swapValue=depositValues[0]/3
):
    swappool = default_swappool_self
    chain.mine(timestamp=chain.time() + 60*60*24*10)

    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappool, swapValue, {"from": base_account})
    assert token2.balanceOf(base_account) == 0

    y = get_swap_return(swapValue, token1, token2, swappool)

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
        0,  # Equal to False, False
        base_account,
        {"from": base_account},
    )
    
    assert token1.balanceOf(base_account) == 0
    assert token2.balanceOf(base_account) == 0
    # Polymerase  ## IBC emulator has no way to get the index.
    # We need to keep tract of the index ourself. As only 1 tx is in the queue, it is 0.
    txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": base_account})
    # Inswap is follows IBC executions.
    assert token2.balanceOf(base_account) > 0

    out = token2.balanceOf(base_account)

    assert token1.balanceOf(base_account) == 0

    check_swap_return(out, y, [swapValue, token1.balanceOf(swappool)])
    
    # Ensure no reverse exploit.
    return_swap_check(swapValue, out, token2, token1, swappool, base_account)
