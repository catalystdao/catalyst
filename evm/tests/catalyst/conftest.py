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
]


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
        selfConnection=False
    )

    