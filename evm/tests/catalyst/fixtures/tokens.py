import pytest

# An easy way to deploy a simple token.
@pytest.fixture(scope="module")
def create_token(project, deployer):
    def create_token(name, symbol, decimals, supply, deployer=deployer):
        return deployer.deploy(project.Token, name, symbol, decimals, supply*10**decimals)

    yield create_token


@pytest.fixture(scope="module")
def tokens_config(raw_config):

    yield raw_config["tokens"]


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
