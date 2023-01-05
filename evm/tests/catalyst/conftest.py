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
def isolation(module_isolation):
    pass


def para_tokens(provided, token_list):
    out_list = []
    for val in provided:
        out_list.append(token_list[val])
    return out_list


# Pool data given to create pools for tests.
@pytest.fixture(scope="module", params=volatile_params[:1], ids=["3 token", "2 token"][:1])
def pool_data(request, token1, token2, token3, token4):
    token_list = [token1, token2, token3, token4]
    return {
        **request.param, 
        "tokens": para_tokens(request.param["tokens"], token_list), 
        "deployer": accounts[request.param.get("deployer")]
    }


@pytest.fixture(scope="module")
def pool_data_cross(token1, token2, token3):
    return [dict(
        amp=2**64,
        tokens = [token1, token2],
        depositAmounts=[10 * 10**18, 1000 * 10**18],
        weights=[1,1,1],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1]
    ), dict(
        amp=2**64,
        tokens = [token3],
        depositAmounts=[1000 * 10**6],
        weights=[1,1,1],
        poolName="POOLNAME2",
        poolSymbol="PS2",
        deployer=accounts[1]
    )]


@pytest.fixture(scope="module")
def amp_pool_data(token1, token2, token3):
    amp = 2**62
    return dict(
        amp=amp,
        tokens = [token1, token2, token3],
        depositAmounts=[10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        weights=[int(int(1000 * 10**18)/int(10 * 10**18)), 1, int(int(1000 * 10**18)/int(1000 * 10**6))],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1],
        selfConnection=True  # TODO: Figure out how to parametrize setting this to 0. (should skip tests which requires local cross-chain swaps)
    )


@pytest.fixture(scope="module")
def amp_pool_data_cross(token1, token2, token3):
    amp = 2**62
    return [dict(
        amp=amp,
        tokens = [token1, token2],
        depositAmounts=[10 * 10**18, 1000 * 10**18],
        weights=[int(int(1000 * 10**18)/int(10 * 10**18)), 1],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1]
    ), dict(
        amp=amp,
        tokens = [token3],
        depositAmounts=[1000 * 10**6],
        weights=[int(int(1000 * 10**18)/int(1000 * 10**6))],
        poolName="POOLNAME2",
        poolSymbol="PS2",
        deployer=accounts[1]
    )]