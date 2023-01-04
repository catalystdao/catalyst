import pytest
from brownie import testFixedPointMath


@pytest.fixture(scope="module")
def mathX64(gov):
    yield gov.deploy(testFixedPointMath)
