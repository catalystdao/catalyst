import pytest
from brownie import FixedPointMathX64


@pytest.fixture(scope="module")
def mathX64(gov):
    yield gov.deploy(FixedPointMathX64)
