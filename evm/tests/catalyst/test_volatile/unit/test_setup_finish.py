import pytest
import brownie
from brownie import (
    ZERO_ADDRESS,
    CatalystSwapPool
)

from tests.catalyst.fixtures.pools import MAX_POOL_ASSETS

@pytest.fixture(scope="module")
def sample_pool(deploy_pool, tokens, deployer, max_pool_assets):
    yield deploy_pool(
        tokens          = tokens[:max_pool_assets],
        token_balances  = [10**8]*max_pool_assets,
        weights         = [1]*max_pool_assets,
        amp             = 10**18,
        name            = "",
        symbol          = "",
        deployer        = deployer,
    )



# Main setup parametrized test **************************************************************************************************
# Test that all provided pool configs work correctly
def test_finish_setup(pool, deployer):

    pool.finishSetup({"from": deployer})

    assert pool.ready()

    # TODO verify that all parameters are saved correctly on-chain



# Authority and state tests *****************************************************************************************************

def test_pool_not_ready(sample_pool):
    assert not sample_pool.ready()


def test_finish_setup_unauthorized(sample_pool, molly):

    with brownie.reverts():
        sample_pool.finishSetup({"from": molly})


def test_finish_setup_twice(sample_pool, deployer):

    sample_pool.finishSetup({"from": deployer})

    with brownie.reverts():
        sample_pool.finishSetup({"from": deployer})


def test_finish_setup_only_local(deploy_pool, tokens, deployer, max_pool_assets):

    sp = deploy_pool(
        tokens          = tokens[:max_pool_assets],
        token_balances  = [10**8]*max_pool_assets,
        weights         = [1]*max_pool_assets,
        amp             = 10**18,
        name            = "",
        symbol          = "",
        deployer        = deployer,
        only_local      = True
    )

    sp.finishSetup({"from": deployer})

    assert sp.onlyLocal()


def test_finish_setup_not_only_local(deploy_pool, tokens, deployer, max_pool_assets):

    sp = deploy_pool(
        tokens          = tokens[:max_pool_assets],
        token_balances  = [10**8]*max_pool_assets,
        weights         = [1]*max_pool_assets,
        amp             = 10**18,
        name            = "",
        symbol          = "",
        deployer        = deployer,
        only_local      = False
    )

    sp.finishSetup({"from": deployer})

    assert not sp.onlyLocal()

