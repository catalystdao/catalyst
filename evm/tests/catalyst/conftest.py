import pytest
from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    ZERO_ADDRESS,
)
from brownie import convert

CHANNEL_ID = 0


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


@pytest.fixture(scope="module")
def swappool_template(gov):
    yield gov.deploy(CatalystSwapPool)


@pytest.fixture(scope="module")
def amplifiedswappool_template(gov):
    yield gov.deploy(CatalystSwapPoolAmplified)


@pytest.fixture(scope="module")
def swapfactory(gov, swappool_template, amplifiedswappool_template):
    yield gov.deploy(
        CatalystSwapPoolFactory, swappool_template, amplifiedswappool_template, 0
    )


@pytest.fixture(scope="module")
def crosschaininterface(gov, swapfactory, ibcemulator):
    cci = gov.deploy(CatalystIBCInterface, swapfactory, ibcemulator, CHANNEL_ID)
    cci.registerPort({"from": gov})  # register port 1
    cci.registerPort({"from": gov})  # register port 2

    yield cci


@pytest.fixture(scope="module")
def chainId(crosschaininterface):

    yield crosschaininterface.chain_id()


@pytest.fixture(scope="module")
def deploy_swappool(accounts, swapfactory, crosschaininterface):
    def deploy_swappool(
        tokens,
        token_balances,
        amp=2**64,
        name="",
        symbol="",
        weights=None,
        deployer=accounts[0],
    ):
        it = 0
        for token in tokens:
            if token == ZERO_ADDRESS:
                break
            token.transfer(deployer, token_balances[it], {"from": accounts[0]})
            token.approve(swapfactory, token_balances[it], {"from": deployer})
            it += 1

        if weights is None:
            weights = [1 for _ in tokens]
        tx_R = swapfactory.deploy_swappool(
            1 if amp != 2**64 else 0,
            tokens,
            token_balances,
            weights,
            amp,
            name,
            symbol,
            crosschaininterface,
            {"from": deployer},
        )

        if amp == 2**64:
            return CatalystSwapPool.at(tx_R.return_value)
        else:
            return CatalystSwapPoolAmplified.at(tx_R.return_value)

    yield deploy_swappool


default_depositValues = [10 * 10**18, 1000 * 10**18, 1000 * 10**6]


@pytest.fixture(scope="module")
def default_swappool(deploy_swappool, token1, token2, token3, accounts):
    depositValues = default_depositValues
    tokens = [token1, token2, token3]
    weights = [1, 1, 1]
    deployer = accounts[1]

    sp = deploy_swappool(
        tokens,
        depositValues,
        2**64,
        "POOLNAME",
        "PS",
        weights=weights,
        deployer=deployer,
    )
    sp.finishSetup({"from": deployer})

    # Validate swappool is correctly created
    assert sp.ready()
    assert sp.balanceOf(deployer) == 2**64
    assert sp.balanceOf(accounts[0]) == 0
    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositValues[i]
        assert token.balanceOf(sp) == depositValue
        assert token.balanceOf(deployer) == 0

    yield sp


@pytest.fixture(scope="module")
def default_swappool_self(deploy_swappool, token1, token2, token3, accounts, chainId):
    TARGET_CHAIN_ID = chainId
    depositValues = default_depositValues
    tokens = [token1, token2, token3]
    weights = [1, 1, 1]
    deployer = accounts[1]

    sp = deploy_swappool(
        tokens,
        depositValues,
        2**64,
        "POOLNAME",
        "PS",
        weights=weights,
        deployer=deployer,
    )

    sp.createConnection(
        TARGET_CHAIN_ID,
        convert.to_bytes(sp.address.replace("0x", "")),
        True,
        {"from": deployer},
    )
    sp.finishSetup({"from": deployer})

    # Validate swappool is correctly created
    assert sp.ready()
    assert sp.balanceOf(deployer) == 2**64
    assert sp.balanceOf(accounts[0]) == 0
    assert sp._chaininterface() != ZERO_ADDRESS
    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositValues[i]
        assert token.balanceOf(sp) == depositValue
        assert token.balanceOf(deployer) == 0

    yield sp


@pytest.fixture(scope="module")
def default_swappool_2(deploy_swappool, token1, token2, accounts):
    depositValues = default_depositValues
    tokens = [token1, token2]
    weights = [1, 1]
    deployer = accounts[1]

    sp = deploy_swappool(
        tokens,
        depositValues,
        2**64,
        "POOLNAME",
        "PS",
        weights=weights,
        deployer=deployer,
    )
    sp.finishSetup({"from": deployer})

    # Validate swappool is correctly created
    assert sp.ready()
    assert sp.balanceOf(deployer) == 2**64
    assert sp.balanceOf(accounts[0]) == 0
    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositValues[i]
        assert token.balanceOf(sp) == depositValue
        assert token.balanceOf(deployer) == 0

    yield sp
