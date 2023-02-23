import pytest


@pytest.fixture(scope="module")
def ibc_emulator(project, gov):
    yield gov.deploy(project.IBCEmulator)


@pytest.fixture(scope="module")
def swap_factory(project, gov):
    yield gov.deploy(project.CatalystSwapPoolFactory, 0)


@pytest.fixture(scope="module")
def volatile_swap_pool_template(project, gov, swap_factory):
    yield gov.deploy(project.CatalystSwapPoolVolatile, swap_factory)


@pytest.fixture(scope="module")
def amplified_swap_pool_template(project, gov, swap_factory):
    yield gov.deploy(project.CatalystSwapPoolAmplified, swap_factory)
    

@pytest.fixture(scope="module")
def cross_chain_interface(project, gov, ibc_emulator):
    cci = gov.deploy(project.CatalystIBCInterface, ibc_emulator)
    cci.registerPort(sender=gov)  # register port 1
    cci.registerPort(sender=gov)  # register port 2

    yield cci


# Parametrized fixtures
@pytest.fixture(scope="module")
def swap_pool_class(project, swap_pool_type):
    if   swap_pool_type == "volatile"  : yield project.CatalystSwapPoolVolatile
    elif swap_pool_type == "amplified" : yield project.CatalystSwapPoolAmplified