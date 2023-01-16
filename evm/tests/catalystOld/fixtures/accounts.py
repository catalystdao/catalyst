import pytest

# The usual deployer of contracts.
@pytest.fixture(scope="module")
def deployer(accounts):
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