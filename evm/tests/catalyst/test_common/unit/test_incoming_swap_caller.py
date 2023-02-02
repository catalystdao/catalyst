import pytest
from brownie import reverts, web3

pytestmark = pytest.mark.usefixtures("pool_connect_itself")

def test_swapFromUnits_must_be_called_by_cci(
    pool,
    berg,
):
    cci = pool._chaininterface()
    
    with reverts():
        pool.swapFromUnits(
            0,
            berg,
            10**16,
            0,
            web3.keccak(text="e"),
            {'from': berg}
        )
    
    pool.swapFromUnits(
        0,
        berg,
        10**16,
        0,
        web3.keccak(text="e"),
        {'from': cci}
    )


def test_inLiquidity_must_be_called_by_cci(
    pool,
    berg,
):
    cci = pool._chaininterface()
    
    with reverts():
        pool.inLiquidity(
            berg,
            10**16,
            0,
            web3.keccak(text="e"),
            {'from': berg}
        )
    
    pool.inLiquidity(
        berg,
        10**16,
        0,
        web3.keccak(text="e"),
        {'from': cci}
    )