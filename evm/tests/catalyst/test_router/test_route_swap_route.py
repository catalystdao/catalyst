import pytest
from brownie import convert

MSG_SENDER = 0x01
ADDRESS_THIS = 0x02
BALANCE_THIS = 2**255

def byte_sum(elements: list):
    b = b""
    for elem in elements:
        b += elem
    return b

@pytest.fixture(scope="module")
def token1(create_token):
    yield create_token("Token1", "TKN1", 18, 10000)

@pytest.fixture(scope="module")
def token2(create_token):
    yield create_token("Token2", "TKN2", 18, 10000)

@pytest.fixture(scope="module")
def token3(create_token):
    yield create_token("Token3", "TKN3", 18, 10000)

@pytest.fixture(scope="module")
def pool1_tkns(weth, token1):
    yield [weth, token1]
    
@pytest.fixture(scope="module")
def pool2_tkns(token2, token3):
    yield [token2, token3]

@pytest.fixture(scope="module")
def pool11(deploypool, pool1_tkns, accounts, weth):
    
    weth.deposit({'from': accounts[0], 'value': 1000*10**18})
    
    yield deploypool(
        tokens=pool1_tkns,
        token_balances=[1000*10**18, 1000*10**18],
        weights=[1, 1],
        amp=10**18,
        name="Pool 11",
        symbol="P11"
    )

@pytest.fixture(scope="module")
def pool12(deploypool, token1):
    yield deploypool(
        tokens=[token1],
        token_balances=[1000*10**18],
        weights=[1],
        amp=10**18,
        name="Pool 12",
        symbol="P12"
    )

@pytest.fixture(scope="module")
def pool21(deploypool, token2):
    yield deploypool(
        tokens=[token2],
        token_balances=[1000*10**18],
        weights=[1],
        amp=10**18,
        name="Pool 21",
        symbol="P21"
    )

@pytest.fixture(scope="module")
def pool22(deploypool, pool2_tkns):
    yield deploypool(
        tokens=pool2_tkns,
        token_balances=[1000*10**18, 1000*10**18],
        weights=[1, 1],
        amp=10**18,
        name="Pool 21",
        symbol="P21"
    )

def test_sendSwap(catalyst_router, pool11, pool12, pool21, pool22, weth, token1, token2, token3, channel_id, ibc_emulator, accounts, berg, encode_router_payload):
    
    pool12.setConnection(channel_id, convert.to_bytes(pool21.address), True, {'from': accounts[0]})
    pool21.setConnection(channel_id, convert.to_bytes(pool12.address), True, {'from': accounts[0]})
    
    amount = 10**18
    
    payload1 = encode_router_payload([0x08, 0x00, 0x04], [
        [ADDRESS_THIS, amount],
        [pool11.address, weth.address, token1.address, amount, 0],
        [pool12.address, channel_id, pool21.address, berg.address, token1.address, 0, BALANCE_THIS, 0, berg.address]  # Leave the calldata param blank.
    ])
    
    tx = catalyst_router.execute(*payload1, {'from': berg.address, 'value': amount})
    
    txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    assert token2.balanceOf(berg) > 998000000000000000


def test_route_route(catalyst_router, pool11, pool12, pool21, pool22, weth, token1, token2, token3, channel_id, ibc_emulator, accounts, berg, encode_router_payload):
    
    pool12.setConnection(channel_id, convert.to_bytes(pool21.address), True, {'from': accounts[0]})
    pool21.setConnection(channel_id, convert.to_bytes(pool12.address), True, {'from': accounts[0]})
    
    amount = 10**18
    
    payload2 = encode_router_payload([0x00, 0x01], [
        [pool22.address, token2.address, token3.address, BALANCE_THIS, 0], 
        [token3.address, berg.address, 0]
    ])
    
    payloadTarget = convert.to_bytes(catalyst_router.address, type_str="bytes32") + convert.to_bytes(catalyst_router.execute.encode_input(*payload2)[10:], "bytes")
    
    payload1 = encode_router_payload([0x05, 0x00, 0x09], [
        [ADDRESS_THIS, amount],
        [pool11.address, weth.address, token1.address, amount, 0],
        [pool12.address, channel_id, pool21.address, catalyst_router.address, token1.address, 0, BALANCE_THIS, 0, berg.address, payloadTarget]
    ])
    
    tx = catalyst_router.execute(*payload1, {'from': berg.address, 'value': amount})
    
    txe = ibc_emulator.execute(tx.events["IncomingMetadata"]["metadata"][0], tx.events["IncomingPacket"]["packet"], {"from": berg})
    
    assert token3.balanceOf(berg) > 997000000000000000