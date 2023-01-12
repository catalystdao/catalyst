import pytest

from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    IBCEmulator
)


@pytest.fixture(scope="module")
def ibc_emulator(deployer):
    yield deployer.deploy(IBCEmulator)


@pytest.fixture(scope="module")
def swap_pool_template(deployer):
    yield deployer.deploy(CatalystSwapPool)


@pytest.fixture(scope="module")
def amplified_swap_pool_template(deployer):
    yield deployer.deploy(CatalystSwapPoolAmplified)
    

@pytest.fixture(scope="module")
def swap_factory(deployer, swap_pool_template, amplified_swap_pool_template):
    yield deployer.deploy(
        CatalystSwapPoolFactory, swap_pool_template, amplified_swap_pool_template, 0
    )


@pytest.fixture(scope="module")
def cross_chain_interface(deployer, swap_factory, ibc_emulator):
    cci = deployer.deploy(CatalystIBCInterface, swap_factory, ibc_emulator)
    cci.registerPort({"from": deployer})  # register port 1
    cci.registerPort({"from": deployer})  # register port 2

    yield cci