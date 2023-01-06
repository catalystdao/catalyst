import pytest
from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    ZERO_ADDRESS,
)
from brownie import convert, accounts
from pool_data import pool_information
from itertools import combinations

CHANNEL_ID = 0

pytest_plugins = [
    "fixtures.accounts",
    "fixtures.common_actions",
    "fixtures.core_contracts",
    "fixtures.tokens",
    "fixtures.functions",
]


# Test isolation
@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


def pytest_addoption(parser):
    parser.addoption("--poolname", help="Only runs tests for poolname")
    parser.addoption("--amppoolname", help="Only runs tests for amppoolname")


def pytest_generate_tests(metafunc):
    volatile_pools = {}
    amplified_pools = {}
    for key, value in pool_information.items():
        if value.get("amp") == 2**64:
            volatile_pools[key] = value
        else:
            amplified_pools[key] = value
    
    
    if "pool_data" in  metafunc.fixturenames:
        poolname = metafunc.config.getoption('poolname')
        if poolname is not None:
            assert 1 < len(volatile_pools[poolname].get("tokens"))
            assert 2**64 == len(volatile_pools[poolname].get("amp"))
            metafunc.parametrize("pool_data", [poolname], indirect=True, scope="session")
        else:
            localSwapPools = []
            for key, value in volatile_pools.items():
                if 1 < len(value.get("tokens")):
                    localSwapPools.append(key)
            metafunc.parametrize("pool_data", localSwapPools, indirect=True, scope="session")
    
    
    if "pool_data_cross" in  metafunc.fixturenames:
        multiSwapPools = []
        ids = []
        for pool1, pool2 in combinations(volatile_pools.items(), 2):
            key1, value1, key2, value2 = pool1 + pool2
            allTokens = value1.get("tokens") + value2.get("tokens")
            if len(set(allTokens)) == len(allTokens):
                multiSwapPools.append([key1, key2])
                ids.append(key1 + "+" + key2)
        metafunc.parametrize("pool_data_cross", multiSwapPools, ids=ids, indirect=True, scope="session")
    
    
    if "amp_pool_data" in  metafunc.fixturenames:
        amppoolname = metafunc.config.getoption('amppoolname')
        if amppoolname is not None:
            assert 1 < len(amplified_pools[amppoolname].get("tokens"))
            metafunc.parametrize("pool_data", [amppoolname], indirect=True, scope="session")
        else:
            localSwapPools = []
            for key, value in amplified_pools.items():
                if 1 < len(value.get("tokens")):
                    localSwapPools.append(key)
            metafunc.parametrize("amp_pool_data", localSwapPools, indirect=True, scope="session")
    
    
    if "amp_pool_data_cross" in  metafunc.fixturenames:
        multiSwapPools = []
        ids = []
        print(list(combinations(amplified_pools.items(), 2)))
        for pool1, pool2 in combinations(amplified_pools.items(), 2):
            key1, value1, key2, value2 = pool1 + pool2
            allTokens = value1.get("tokens") + value2.get("tokens")
            if len(set(allTokens)) == len(allTokens):
                multiSwapPools.append([key1, key2])
                ids.append(key1 + "+" + key2)
        metafunc.parametrize("amp_pool_data_cross", multiSwapPools, ids=ids, indirect=True, scope="session")
            

# Pool data given to create pools for tests.
@pytest.fixture(scope="session")
def pool_data(request):
    base_data = pool_information[request.param]
    return {
        **base_data, 
        "deployer": accounts[base_data.get("deployer")]
    }


@pytest.fixture(scope="session")
def pool_data_cross(request):
    pool1, pool2 = request.param
    base_data1, base_data2 = pool_information[pool1], pool_information[pool2]
    return [{
        **base_data1, 
        "deployer": accounts[base_data1.get("deployer")]
    }, {
        **base_data2, 
        "deployer": accounts[base_data2.get("deployer")]
    }]


@pytest.fixture(scope="session")
def amp_pool_data(request):
    base_data = pool_information[request.param]
    return {
        **base_data, 
        "deployer": accounts[base_data.get("deployer")]
    }


@pytest.fixture(scope="session")
def amp_pool_data_cross(request):
    pool1, pool2 = request.param
    base_data1, base_data2 = pool_information[pool1], pool_information[pool2]
    return [{
        **base_data1, 
        "deployer": accounts[base_data1.get("deployer")]
    }, {
        **base_data2, 
        "deployer": accounts[base_data2.get("deployer")]
    }]