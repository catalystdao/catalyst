import pytest
from brownie import reverts, web3

pytestmark = pytest.mark.usefixtures("pool_connect_itself")

def test_receiveSwap_must_be_called_by_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts():
        pool.receiveSwap(
            0,
            berg,
            10**16,
            0,
            web3.keccak(text="e"),
            {'from': berg}
        )
    
    pool.receiveSwap(
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
    cci = pool._chainInterface()
    
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
    

def test_release_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts("dev: Only _chainInterface"):
        pool.releaseEscrowACK(
            web3.keccak(text="e"),
            0,
            0,
            berg,
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts("dev: Invalid messageHash. Alt: Escrow doesn't exist."):
        pool.releaseEscrowACK(
            web3.keccak(text="e"),
            0,
            0,
            berg,
            {'from': cci}
        )
        
def test_timeout_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts("dev: Only _chainInterface"):
        pool.releaseEscrowTIMEOUT(
            web3.keccak(text="e"),
            0,
            0,
            berg,
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts("dev: Invalid messageHash. Alt: Escrow doesn't exist."):
        pool.releaseEscrowTIMEOUT(
            web3.keccak(text="e"),
            0,
            0,
            berg,
            {'from': cci}
        )


def test_release_liquidity_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts("dev: Only _chainInterface"):
        pool.releaseLiquidityEscrowACK(
            web3.keccak(text="e"),
            0,
            0,
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts("dev: Invalid messageHash. Alt: Escrow doesn't exist."):
        pool.releaseLiquidityEscrowACK(
            web3.keccak(text="e"),
            0,
            0,
            {'from': cci}
        )
    

def test_timeout_liquidity_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts("dev: Only _chainInterface"):
        pool.releaseLiquidityEscrowTIMEOUT(
            web3.keccak(text="e"),
            0,
            0,
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts("dev: Invalid messageHash. Alt: Escrow doesn't exist."):
        pool.releaseLiquidityEscrowTIMEOUT(
            web3.keccak(text="e"),
            0,
            0,
            {'from': cci}
        )