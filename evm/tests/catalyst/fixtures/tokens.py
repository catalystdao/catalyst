import pytest
from brownie import Token

# An easy way to deploy a simple token.
@pytest.fixture(scope="module")
def create_token(deployer):
    def create_token(name, symbol, decimals, supply, deployer=deployer):
        return Token.deploy(name, symbol, decimals, supply*10**decimals, {"from": deployer})

    yield create_token


@pytest.fixture(scope="module")
def tokens_config(raw_config):

    raw_tokens_config = raw_config["tokens"]

    assert len(raw_tokens_config) >= 4, "At least 4 tokens must be defined on the test config file"

    # Verify the tokens config
    for config in raw_tokens_config:
        assert "name" in config and isinstance(config["name"], str)
        assert "symbol" in config and isinstance(config["symbol"], str)
        assert "decimals" in config and isinstance(config["decimals"], int)
        assert "supply" in config and isinstance(config["supply"], int) and config["supply"] > 0

    yield raw_tokens_config


@pytest.fixture(scope="module")
def tokens(tokens_config, create_token):

    yield [
        create_token(
            token_config["name"],
            token_config["symbol"],
            token_config["decimals"],
            token_config["supply"]
        ) for token_config in tokens_config
    ]

