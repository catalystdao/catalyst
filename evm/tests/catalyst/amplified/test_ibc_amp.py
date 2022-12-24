import brownie
import numpy as np
import pytest
from brownie.test import given, strategy
from hypothesis import settings


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


POOLNAME = "PS OneTwoThree"
POOLSYMBOL = "ps(OTT) "


depositValues = [1000 * 10**18, 1000 * 10**18, 1000 * 10**18]


@pytest.fixture(scope="module")
def swappool(accounts, gov, swapfactory, deploy_swappool, crosschaininterface, token1, token2, token3):
    base_account = accounts[2]

    tokens = [token1, token2, token3]

    sp = deploy_swappool(
        [token1, token2, token3],
        depositValues,
        2**63,
        POOLNAME,
        POOLSYMBOL,
        deployer=base_account,
    )
    
    TARGET_CHAIN_ID = crosschaininterface.chain_id()
    sp.createConnectionWithChain(TARGET_CHAIN_ID, brownie.convert.to_bytes(sp.address.replace("0x", "")), True, {"from": base_account})
    sp.finishSetup({"from": base_account})

    # Validate swappool is correctly created
    assert sp.ready()
    assert sp.balanceOf(base_account) == 2**64
    assert sp.balanceOf(gov) == 0
    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositValues[i]
        assert token.balanceOf(sp) == depositValue
        assert token.balanceOf(base_account) == 0

    yield sp


@pytest.mark.no_call_coverage
@given(swapValue=strategy("uint256", max_value=depositValues[0], min_value=10**18))
def test_ibc_swap(
    accounts, token1, token2, gov, swappool, ibcemulator, crosschaininterface, swapValue
):
    # swapValue = 10**18 * 5

    # This function is used for debugging.
    def setup():
        token1.transfer(base_account, token1.balanceOf(gov), {"from": gov})
        token1.approve(swappool, 2**256 - 1, {"from": base_account})


    # token1 to token2
    base_account = accounts[2]
    token1.transfer(base_account, swapValue, {"from": gov})
    token1.approve(swappool, swapValue, {"from": base_account})
    assert token2.balanceOf(base_account) == 0

    a = token1.balanceOf(swappool)
    b = token2.balanceOf(swappool)
    x = swapValue
    w1 = swappool._weight(token1)
    w2 = swappool._weight(token2)
    w = w1 / w2
    k = swappool._amp() / 2**64
    y = b - ((w2 * b ** (1 - k) - w1 * ((a + x) ** (1 - k) - a ** (1 - k))) / (w2)) ** (
        1 / (1 - k)
    )

    B0 = a

    diviation = 0.02 / 100

    # swapToUnits
    # (_chain : uint256, _targetPool : bytes32, _fromAsset : address, _toAsset : uint256, _who : bytes32, _amount : uint256) -> uint256
    tokenArr = [
        swappool._tokenIndexing(0),
        swappool._tokenIndexing(1),
        swappool._tokenIndexing(2),
    ]
    toAssetIndex = tokenArr.index(token2)
    assert swappool._tokenIndexing(toAssetIndex) == token2

    # When using polymerase emulator, the target chain should be the same as the
    # sending chain. We get the sending chain using chain.id. However, this does not return
    # the true value when using mainnet_fork. For that reason, we implement the chain_id
    # function in CCI and get the chain id from that. This will be equal to the connrection
    # created by the catalyst template.
    TARGET_CHAIN_ID = crosschaininterface.chain_id()

    tx = swappool.swapToUnits(
        TARGET_CHAIN_ID,
        brownie.convert.to_bytes(swappool.address.replace("0x", "")),
        brownie.convert.to_bytes(base_account.address.replace("0x", "")),
        token1,
        toAssetIndex,
        swapValue,
        0,
        0,  # Equal to False, False,
        base_account,
        {"from": base_account},
    )
    assert token1.balanceOf(base_account) == 0
    assert token2.balanceOf(base_account) == 0
    U = tx.return_value / 2**64
    calcU = w1 * ((a + x) ** (1 - k) - a ** (1 - k))
    outAccordingToCalcU = b - ((w2 * b ** (1 - k) - calcU) / (w2)) ** (1 / (1 - k))
    # Polymerase  ## IBC emulator has no way to get the index.
    # We need to keep tract of the index ourself. As only 1 tx is in the queue, it is 0.
    txe = ibcemulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": base_account})
    # Inswap is follows IBC executions.
    assert token2.balanceOf(base_account) > 0

    out = token2.balanceOf(base_account)

    assert token1.balanceOf(base_account) == 0

    if (swapValue < token1.balanceOf(swappool) / 1000) or (
        y == 0
    ):  # also covers swapValue == 0
        assert token2.balanceOf(base_account) <= y
    else:
        print(token2.balanceOf(base_account) / y)
        assert (
            1 + diviation >= token2.balanceOf(base_account) / y >= 1 - diviation * 100
        )  # lower is 2%
        # The calculation should get more precise as swapValue goes up.

    # swap the other way
    token2.approve(swappool, out, {"from": base_account})
    tx2 = swappool.localswap(token2, token1, out, 0, {"from": base_account})

    assert token2.balanceOf(base_account) == 0
    out2 = token1.balanceOf(base_account)

    assert out2 <= swapValue
    print(out2 / swapValue, out)
