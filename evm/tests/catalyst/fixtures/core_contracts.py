import pytest

from brownie import (
    CatalystSwapPoolFactory,
    CatalystSwapPool,
    CatalystSwapPoolAmplified,
    CatalystIBCInterface,
    IBCEmulator,
    ZERO_ADDRESS,
    convert
)


@pytest.fixture(scope="module")
def ibcemulator(deployer):
    yield deployer.deploy(IBCEmulator)


@pytest.fixture(scope="module")
def swappool_template(deployer):
    yield deployer.deploy(CatalystSwapPool)


@pytest.fixture(scope="module")
def amplifiedswappool_template(deployer):
    yield deployer.deploy(CatalystSwapPoolAmplified)
    

@pytest.fixture(scope="module")
def swapfactory(deployer, swappool_template, amplifiedswappool_template):
    yield deployer.deploy(
        CatalystSwapPoolFactory, swappool_template, amplifiedswappool_template, 0
    )


@pytest.fixture(scope="module")
def crosschaininterface(deployer, swapfactory, ibcemulator):
    cci = deployer.deploy(CatalystIBCInterface, swapfactory, ibcemulator)
    cci.registerPort({"from": deployer})  # register port 1
    cci.registerPort({"from": deployer})  # register port 2

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


@pytest.fixture(scope="session")
def channelId():
    yield convert.to_bytes(1, type_str="bytes32")


# Non amplified pools
@pytest.fixture(scope="module")
def swappool(channelId, deploy_swappool, pool_data):
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
            channelId,
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
def amp_swappool(channelId, deploy_swappool, amp_pool_data):
    assert amp_pool_data.get("amp") < 2**64
    deployer = amp_pool_data.get("deployer")
    tokens = amp_pool_data.get("tokens")
    depositAmounts = amp_pool_data.get("depositAmounts")
    

    sp = deploy_swappool(
        tokens,
        depositAmounts,
        amp_pool_data.get("weights"),
        amp_pool_data.get("amp"),
        amp_pool_data.get("poolName"),
        amp_pool_data.get("poolSymbol"),
        deployer=deployer,
    )
    if amp_pool_data.get("selfConnection"):
        sp.createConnection(
            channelId,
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


