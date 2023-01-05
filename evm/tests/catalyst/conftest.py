import pytest
from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    ZERO_ADDRESS,
)
from brownie import convert, accounts

CHANNEL_ID = 0

pytest_plugins = [
    "fixtures.accounts",
    "fixtures.core_contracts",
    "fixtures.tokens"
    "fixtures.functions"
]



# Pool data given to create pools for tests.

@pytest.fixture(scope="module")
def pool_data(token1, token2, token3):
    return dict(
        amp=2*64,
        tokens = [token1, token2, token3],
        depositAmounts=[10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        weights=[1,1,1],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1],
        selfConnection=True  # TODO: Figure out how to parametrize setting this to 0. (should skip tests which requires local cross-chain swaps)
    )

@pytest.fixture(scope="module")
def amp_pool_data(token1, token2, token3):
    return dict(
        amp=2*62,
        tokens = [token1, token2, token3],
        depositAmounts=[10 * 10**18, 1000 * 10**18, 1000 * 10**6],
        weights=[int(int(1000 * 10**18)/int(10 * 10**6)), 1, int(int(1000 * 10**18)/int(1000 * 10**6))],
        poolName="POOLNAME",
        poolSymbol="PS",
        deployer=accounts[1],
        selfConnection=True  # TODO: Figure out how to parametrize setting this to 0. (should skip tests which requires local cross-chain swaps)
    )
    