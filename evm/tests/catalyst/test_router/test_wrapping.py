
from brownie import convert

MSG_SENDER = 0x01
ADDRESS_THIS = 0x02


def test_wrap_unwrap(catalyst_router, weth, deployer, permit2, encode_router_payload):
     
    amount = 10**18
    
    payload = encode_router_payload([0x08, 0x04], [[ADDRESS_THIS, amount], [weth.address, MSG_SENDER, amount]])
    
    tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == amount

    weth.approve(permit2, amount, {'from': deployer})
    permit2.approve(weth, catalyst_router, 2**160-1, 2**48-1, {'from': deployer})

    payload = encode_router_payload([0x02, 0x09], [[weth.address, ADDRESS_THIS, amount], [MSG_SENDER, amount]])
    
    tx2 = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == 0
    
    assert tx2.value == amount
    
    

