
from brownie import convert

MSG_SENDER = 0x01
ADDRESS_THIS = 0x02


def test_wrap_unwrap(catalyst_router, weth, deployer, encode_router_payload):
     
    amount = 10**18
    
    payload = encode_router_payload([0x05, 0x01], [[ADDRESS_THIS, amount], [weth.address, MSG_SENDER, amount]])
    
    tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == amount

    weth.approve(catalyst_router, amount, {'from': deployer})

    payload = encode_router_payload([0x08, 0x06], [[weth.address, amount], [MSG_SENDER, amount]])
    
    tx2 = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == 0
    
    assert tx2.value == amount
    
    

