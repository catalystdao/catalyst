import pytest
from brownie import Token


# An easy way to deploy a simple token.
@pytest.fixture(scope="module")
def create_token(gov):
    def create_token(name="TokenName", symbol="TKN", decimal=18, deployer=gov):
        return Token.deploy(name, symbol, decimal, 10000*10**decimal, {"from": deployer})

    yield create_token


@pytest.fixture(scope="module")
def token1(create_token):
    yield create_token("one", "T")


@pytest.fixture(scope="module")
def token2(create_token):
    yield create_token(
        "very long name that is exactly 64 chars. When will this end,here",
        "SYMBOL THAT IS JUST 32 CHAR LONG",
    )


@pytest.fixture(scope="module")
def token3(create_token):
    yield create_token("three", "TTT", decimal=6)


@pytest.fixture(scope="module")
def token4(create_token):
    yield create_token("four", "TTTT", decimal=18)


@pytest.fixture(scope="module")
def tokens(token1, token2, token3, token4):
    yield [token1, token2, token3, token4]