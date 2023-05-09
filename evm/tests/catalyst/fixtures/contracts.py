import pytest

from brownie import (
    CatalystVaultFactory,
    CatalystVaultVolatile,
    CatalystVaultAmplified,
    CatalystIBCInterface,
    IBCEmulator,
)


@pytest.fixture(scope="module")
def ibc_emulator(deployer):
    yield deployer.deploy(IBCEmulator)


@pytest.fixture(scope="module")
def swap_factory(deployer):
    yield deployer.deploy(CatalystVaultFactory, 0)


@pytest.fixture(scope="module")
def volatile_swap_vault_template(deployer, swap_factory):
    yield deployer.deploy(CatalystVaultVolatile, swap_factory)


@pytest.fixture(scope="module")
def amplified_swap_vault_template(deployer, swap_factory):
    yield deployer.deploy(CatalystVaultAmplified, swap_factory)


@pytest.fixture(scope="module")
def cross_chain_interface(deployer, ibc_emulator):
    cci = deployer.deploy(CatalystIBCInterface, ibc_emulator)
    cci.registerPort({"from": deployer})  # register port 1
    cci.registerPort({"from": deployer})  # register port 2

    yield cci


# Parametrized fixtures
@pytest.fixture(scope="module")
def swap_vault_class(swap_vault_type):
    if swap_vault_type == "volatile":
        yield CatalystVaultVolatile
    elif swap_vault_type == "amplified":
        yield CatalystVaultAmplified
