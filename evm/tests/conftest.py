# These fest fixtures define the general test fixtures for all tests.
# if possible, these text fixtures should be used over specific
# implementation, as to easily cross validate if changes to one contract is compatible with other contracts.

import pytest
from brownie import Token, PolymeraseEmulator, IBCEmulator


# A contract with goverance like permissions.
@pytest.fixture(scope="module")
def gov(accounts):
    yield accounts[0]


# The first user
@pytest.fixture(scope="module")
def molly(accounts):
    yield accounts[1]


# The second user
@pytest.fixture(scope="module")
def berg(accounts):
    yield accounts[2]


# The third user
@pytest.fixture(scope="module")
def elwood(accounts):
    yield accounts[3]


# Provies an easy way to deploy a simple token.
# Uses the Polymer token as a reference implementation
@pytest.fixture(scope="module")
def create_token(gov):
    def create_token(name="TokenName", symbol="TKN", decimal=18, deployer=gov):
        return Token.deploy(name, symbol, decimal, 10000*10**18/10**decimal, {"from": deployer})
        # return gov.deploy(Polymer, name, symbol, decimal, 10000, gov)

    yield create_token


# The Polymer Token. gov can mint more.
@pytest.fixture(scope="module")
def polymer(gov):
    yield Token.deploy("TEST POLYMER", "TSC", 18, 10000, {"from": gov})
    #  yield gov.deploy(Polymer, "TEST POLYMER", "TSC", 18, 10000, gov)


#
# Polymerase
#


@pytest.fixture(scope="module")
def polymeraseemulator(gov):
    yield PolymeraseEmulator.deploy({"from": gov})


@pytest.fixture(scope="module")
def ibcemulator(gov):
    yield gov.deploy(IBCEmulator)
