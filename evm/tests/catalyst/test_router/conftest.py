import pytest
from brownie import CatalystRouter, WETH9

@pytest.fixture(scope="module")
def weth(deployer):
    yield WETH9.deploy({'from': deployer})
    
@pytest.fixture(scope="module")
def catalyst_router(weth, deployer):
    yield CatalystRouter.deploy([weth], {'from': deployer})

