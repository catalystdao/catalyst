import pytest
from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    ZERO_ADDRESS,
)
from brownie import convert, accounts
from volatile.volatile_pools import volatile_params

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



def pytest_generate_tests(metafunc):
    if "pool_data" in  metafunc.fixturenames:
        metafunc.parametrize("pool_data", list(volatile_params.keys()), indirect=True, scope="session")


# Pool data given to create pools for tests.
@pytest.fixture(scope="session")
def pool_data(request):
    base_data = volatile_params[request.param]
    return {
        **base_data, 
        "deployer": accounts[base_data.get("deployer")]
    }


@pytest.fixture(scope="session")
def pool_data_cross():
    return [dict(
        amp=2**64,
        tokens = [0, 1],
        depositAmounts=[10 * 10**18, 1000 * 10**18],
        weights=[1,1,1],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1]
    ), dict(
        amp=2**64,
        tokens = [2],
        depositAmounts=[1000 * 10**6],
        weights=[1,1,1],
        poolName="POOLNAME2",
        poolSymbol="PS2",
        deployer=accounts[1]
    )]


@pytest.fixture(scope="session")
def amp_pool_data():
    amp = 2**62
    return dict(
        amp=amp,
        tokens = [0, 1, 2],
        depositAmounts=[10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        weights=[int(int(1000 * 10**18)/int(10 * 10**18)), 1, int(int(1000 * 10**18)/int(1000 * 10**6))],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1],
        selfConnection=True  # TODO: Figure out how to parametrize setting this to 0. (should skip tests which requires local cross-chain swaps)
    )


@pytest.fixture(scope="session")
def amp_pool_data_cross():
    amp = 2**62
    return [dict(
        amp=amp,
        tokens = [0, 1],
        depositAmounts=[10 * 10**18, 1000 * 10**18],
        weights=[int(int(1000 * 10**18)/int(10 * 10**18)), 1],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1]
    ), dict(
        amp=amp,
        tokens = [2],
        depositAmounts=[1000 * 10**6],
        weights=[int(int(1000 * 10**18)/int(1000 * 10**6))],
        poolName="POOLNAME2",
        poolSymbol="PS2",
        deployer=accounts[1]
    )]