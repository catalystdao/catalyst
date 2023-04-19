from constants import MSG_SENDER, ADDRESS_THIS, BALANCE_THIS
from brownie import accounts


def test_transfer(catalyst_router, weth, deployer, encode_router_payload):
    account_transfer = accounts[1]
     
    amount = 10**18
    
    # Actions: Wrap ETH, transfer to account, sweep. Then check that exactly a certain amount is transfered.
    payload = encode_router_payload([0x08, 0x05, 0x04], [[ADDRESS_THIS, amount], [weth.address, account_transfer.address, int(amount/2)], [weth.address, MSG_SENDER, int(amount/2)]])
    
    tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(account_transfer) == amount/2
    assert weth.balanceOf(deployer) == amount/2
    

def test_pay_portion(catalyst_router, weth, deployer, encode_router_payload):
    account_transfer = accounts[1]
     
    amount = 10**18
    bips = 1000  # 10%
    
    # Actions: Wrap ETH, transfer to account, sweep. Then check that exactly a certain amount is transfered.
    payload = encode_router_payload([0x08, 0x06, 0x04], [[ADDRESS_THIS, amount], [weth.address, account_transfer.address, bips], [weth.address, MSG_SENDER, amount - int(amount/10)]])
    
    tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(account_transfer) == int(amount/10)
    assert weth.balanceOf(deployer) == amount - int(amount/10)
    
    

