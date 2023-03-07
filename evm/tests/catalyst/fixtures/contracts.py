import pytest

from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPoolVolatile,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    IBCEmulator
)


@pytest.fixture(scope="module")
def ibc_emulator(deployer):
    yield deployer.deploy(IBCEmulator)


@pytest.fixture(scope="module")
def swap_factory(deployer):
    yield deployer.deploy(CatalystSwapPoolFactory, 0)


@pytest.fixture(scope="module")
def volatile_swap_pool_template(deployer, swap_factory):
    yield deployer.deploy(CatalystSwapPoolVolatile, swap_factory)


@pytest.fixture(scope="module")
def amplified_swap_pool_template(deployer, swap_factory):
    yield deployer.deploy(CatalystSwapPoolAmplified, swap_factory)
    

@pytest.fixture(scope="module")
def cross_chain_interface(deployer, ibc_emulator):
    cci = deployer.deploy(CatalystIBCInterface, ibc_emulator)
    cci.registerPort({"from": deployer})  # register port 1
    cci.registerPort({"from": deployer})  # register port 2

    yield cci


# Parametrized fixtures
@pytest.fixture(scope="module")
def swap_pool_class(swap_pool_type):
    if   swap_pool_type == "volatile"  : yield CatalystSwapPoolVolatile
    elif swap_pool_type == "amplified" : yield CatalystSwapPoolAmplified