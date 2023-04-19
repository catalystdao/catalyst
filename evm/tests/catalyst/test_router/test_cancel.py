from constants import MSG_SENDER, ADDRESS_THIS, BALANCE_THIS
from brownie import accounts, convert, reverts


def test_no_cancel(catalyst_router, weth, deployer, encode_router_payload):
    account_transfer = accounts[1]
     
    amount = 10**18
    
    # Actions: Wrap ETH, transfer to account, sweep. Then check that exactly a certain amount is transfered.
    payload = encode_router_payload([0x0d, 0x08, 0x04], [[deployer.address, convert.to_bytes(123)], [ADDRESS_THIS, amount], [weth.address, MSG_SENDER, int(amount/2)]])
    
    tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == amount
    

def test_cancel(catalyst_router, weth, deployer, encode_router_payload):
    account_transfer = accounts[1]
     
    amount = 10**18
    
    # Actions: Wrap ETH, transfer to account, sweep. Then check that exactly a certain amount is transfered.
    payload = encode_router_payload([0x0d, 0x08, 0x04], [[deployer.address, convert.to_bytes(123)], [ADDRESS_THIS, amount], [weth.address, MSG_SENDER, int(amount/2)]])
    
    catalyst_router.cancelSwap(convert.to_bytes(123), {'from': deployer})
    
    with reverts("typed error: 0x599be0bf"):
        tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    
    
    

