
from brownie import convert

MSG_SENDER = 0x01
ADDRESS_THIS = 0x02


def encode_router_payload(commands: list, parameters: list):
    encoded_commands = b""
    encoded_parameters = []
    for command, parameter in zip(commands, parameters):
        encoded_commands += convert.to_bytes(command, type_str="bytes1")
        encoded_parameter = b""
        for param in parameter:
            encoded_parameter += convert.to_bytes(param, type_str="bytes32")
        encoded_parameters.append(encoded_parameter)
    
    return [encoded_commands, encoded_parameters]


def test_wrap_unwrap(catalyst_router, weth, deployer):
     
    amount = 10**18
    
    payload = encode_router_payload([0x05, 0x01], [[ADDRESS_THIS, amount], [weth.address, MSG_SENDER, amount]])
    
    tx = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == amount

    weth.approve(catalyst_router, amount, {'from': deployer})

    payload = encode_router_payload([0x08, 0x06], [[weth.address, amount], [MSG_SENDER, amount]])
    
    tx2 = catalyst_router.execute(*payload, {'from': deployer, 'value': amount})
    
    assert weth.balanceOf(deployer) == 0
    
    assert tx2.value == amount
    
    

