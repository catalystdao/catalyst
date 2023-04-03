import pytest
from brownie import reverts, web3, convert

pytestmark = [
    pytest.mark.usefixtures("pool_connect_itself"),
    pytest.mark.no_pool_param
]

def test_receiveAsset_must_be_called_by_cci(
    channel_id,
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts():
        pool.receiveAsset(
        channel_id,
            pool.address,  # Use self as source pool
            0,
            berg,
            10**16,
            0,
            web3.keccak(text="e"),
            {'from': berg}
        )
    
    pool.receiveAsset(
        channel_id,
        pool.address,  # Use self as source pool
        0,
        berg,
        10**16,
        0,
        web3.keccak(text="e"),
        {'from': cci}
    )


def test_receiveLiquidity_must_be_called_by_cci(
    channel_id,
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts():
        pool.receiveLiquidity(
            channel_id,
            pool.address,  # Use self as source pool
            berg,
            10**16,
            0,
            0,
            web3.keccak(text="e"),
            {'from': berg}
        )
    
    pool.receiveLiquidity(
        channel_id,
        pool.address,  # Use self as source pool
        berg,
        10**16,
        0,
        0,
        web3.keccak(text="e"),
        {'from': cci}
    )
    

def test_release_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts(): #"dev: Only _chainInterface"
        pool.sendAssetAck(
            berg.address,
            0,
            0,
            berg,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts(): #"dev: Invalid swapHash. Alt: Escrow doesn't exist."
        pool.sendAssetAck(
            berg.address,
            0,
            0,
            berg,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': cci}
        )
        
def test_timeout_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts(): #"dev: Only _chainInterface"
        pool.sendAssetTimeout(
            berg.address,
            0,
            0,
            berg,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts(): #"dev: Invalid swapHash. Alt: Escrow doesn't exist."
        pool.sendAssetTimeout(
            berg.address,
            0,
            0,
            berg,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': cci}
        )


def test_release_liquidity_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts(): #"dev: Only _chainInterface"
        pool.sendLiquidityAck(
            berg.address,
            0,
            0,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts(): #"dev: Invalid swapHash. Alt: Escrow doesn't exist."
        pool.sendLiquidityAck(
            berg.address,
            0,
            0,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': cci}
        )
    

def test_timeout_liquidity_escrow_must_be_called_cci(
    pool,
    berg,
):
    cci = pool._chainInterface()
    
    with reverts(): #"dev: Only _chainInterface"
        pool.sendLiquidityTimeout(
            berg.address,
            0,
            0,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': berg}
        )
    
    # Since no swap has been executed, the escrow hash doesn't exist. However,
    # we still want to check that we can get past the above requirement using
    # a valid sender.
    with reverts(): #"dev: Invalid swapHash. Alt: Escrow doesn't exist."
        pool.sendLiquidityTimeout(
            berg.address,
            0,
            0,
            convert.to_bytes(0, type_str="bytes32"),
            {'from': cci}
        )