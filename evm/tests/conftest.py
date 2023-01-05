# These fest fixtures define the general test fixtures for all tests.
# if possible, these text fixtures should be used over specific
# implementation, as to easily cross validate if changes to one contract is compatible with other contracts.

import pytest
from brownie import Token, IBCEmulator






#
# Polymerase
#


@pytest.fixture(scope="module")
def ibcemulator(gov):
    yield gov.deploy(IBCEmulator)
