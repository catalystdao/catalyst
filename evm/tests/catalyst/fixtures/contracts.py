import pytest

from brownie import (
    CatalystFactory,
    CatalystVaultVolatile,
    CatalystVaultAmplified,
    CatalystIBCInterface,
    IBCEmulator,
    CatalystMathVol,
    CatalystMathAmp,
    CatalystDescriber,
    CatalystDescriberRegistry
)


@pytest.fixture(scope="module")
def ibc_emulator(deployer):
    yield deployer.deploy(IBCEmulator)


@pytest.fixture(scope="module")
def swap_factory(deployer):
    yield deployer.deploy(CatalystFactory, 0)


@pytest.fixture(scope="module")
def math_lib_vol(deployer):
    yield deployer.deploy(CatalystMathVol)
    
@pytest.fixture(scope="module")
def math_lib_amp(deployer):
    yield deployer.deploy(CatalystMathAmp)


@pytest.fixture(scope="module")
def volatile_swap_vault_template(deployer, swap_factory, math_lib_vol):
    yield deployer.deploy(CatalystVaultVolatile, swap_factory, math_lib_vol)


@pytest.fixture(scope="module")
def amplified_swap_vault_template(deployer, swap_factory, math_lib_amp):
    yield deployer.deploy(CatalystVaultAmplified, swap_factory, math_lib_amp)


@pytest.fixture(scope="module")
def cross_chain_interface(deployer, ibc_emulator):
    cci = deployer.deploy(CatalystIBCInterface, ibc_emulator)
    cci.registerPort({"from": deployer})  # register port 1
    cci.registerPort({"from": deployer})  # register port 2

    yield cci


@pytest.fixture(scope="module")
def catalyst_describer_blank(deployer):
    yield deployer.deploy(CatalystDescriber)


@pytest.fixture(scope="module")
def catalyst_describer_filled(deployer, catalyst_describer_blank, volatile_swap_vault_template, amplified_swap_vault_template, cross_chain_interface):
    catalyst_describer_blank.add_whitelisted_template(volatile_swap_vault_template, 1, {'from': deployer})
    catalyst_describer_blank.add_whitelisted_template(amplified_swap_vault_template, 1, {'from': deployer})
    
    catalyst_describer_blank.add_whitelisted_cii(cross_chain_interface, {'from': deployer})
    
    yield catalyst_describer_blank


@pytest.fixture(scope="module")
def catalyst_describer_registry_filled(deployer, catalyst_describer_filled):
    catalyst_registry = deployer.deploy(CatalystDescriberRegistry)
    
    catalyst_registry.add_describer(catalyst_describer_filled, {'from': deployer})

    yield catalyst_registry


# Parametrized fixtures
@pytest.fixture(scope="module")
def swap_vault_class(swap_vault_type):
    if swap_vault_type == "volatile":
        yield CatalystVaultVolatile
    elif swap_vault_type == "amplified":
        yield CatalystVaultAmplified
