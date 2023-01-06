import pytest
from brownie import testFixedPointMath


@pytest.fixture(autouse=True)
def isolation(fn_isolation):
    pass


@pytest.fixture(scope="module")
def mathX64(gov):
    yield gov.deploy(testFixedPointMath)
