import pytest

from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    ZERO_ADDRESS,
    convert
)


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
    cci = gov.deploy(CatalystIBCInterface, swapfactory, ibcemulator)
    cci.registerPort({"from": gov})  # register port 1
    cci.registerPort({"from": gov})  # register port 2

    yield cci


@pytest.fixture(scope="module")
def deploy_swappool(accounts, swapfactory, crosschaininterface):
    def deploy_swappool(
        tokens,
        token_balances,
        weights,
        amp,
        name,
        symbol,
        deployer,
    ):
        it = 0
        for token in tokens:
            if token == ZERO_ADDRESS:
                break
            token.transfer(deployer, token_balances[it], {"from": accounts[0]})
            token.approve(swapfactory, token_balances[it], {"from": deployer})
            it += 1

        tx_R = swapfactory.deploy_swappool(
            1 if amp != 2**64 else 0,  # Template index. 0 is volatile, 1 is amplified.
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
matching_amplified_weights = [default_depositValues[1]/default_depositValues[0], 1, default_depositValues[2]/default_depositValues[0]]


# Non amplified pools
@pytest.fixture(scope="module")
def swappool(deploy_swappool, pool_data):
    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = pool_data.get("tokens")
    depositAmounts = pool_data.get("depositAmounts")

    sp = deploy_swappool(
        tokens,
        depositAmounts,
        pool_data.get("weights"),
        2**64,
        pool_data.get("poolName"),
        pool_data.get("poolSymbol"),
        deployer=deployer,
    )
    
    if pool_data.get("selfConnection"):
        sp.createConnection(
            b"",
            convert.to_bytes(sp.address.replace("0x", "")),
            True,
            {"from": deployer},
        )
    sp.finishSetup({"from": deployer})

    # Validate swappool is correctly created
    assert sp.ready()
    assert sp.balanceOf(deployer) == 2**64
    assert sp.totalSupply() == 2**64
    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositAmounts[i]
        assert token.balanceOf(sp) == depositValue

    yield sp


# Amplified pool
@pytest.fixture(scope="module")
def amp_swappool(deploy_swappool, pool_data):
    assert pool_data.get("amp") < 2**64
    deployer = pool_data.get("deployer")
    tokens = pool_data.get("tokens")
    depositAmounts = pool_data.get("depositAmounts")
    

    sp = deploy_swappool(
        tokens,
        depositAmounts,
        pool_data.get("weights"),
        pool_data.get("amp"),
        pool_data.get("poolName"),
        pool_data.get("poolSymbol"),
        deployer=deployer,
    )
    if pool_data.get("selfConnection"):
        sp.createConnection(
            b"",
            convert.to_bytes(sp.address.replace("0x", "")),
            True,
            {"from": deployer},
        )
    sp.finishSetup({"from": deployer})

    # Validate swappool is correctly created
    assert sp.ready()
    assert sp.balanceOf(deployer) == 2**64
    assert sp.totalSupply() == 2**64
    for i in range(len(tokens)):
        token = tokens[i]
        depositValue = depositAmounts[i]
        assert token.balanceOf(sp) == depositValue

    yield sp
