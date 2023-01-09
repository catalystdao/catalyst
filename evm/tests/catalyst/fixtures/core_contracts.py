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

#TODO set as pytest option?
@pytest.fixture(scope="session")
def swappool_max_assets():
    return 3

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
        only_local = False,
        template_index = None
    ):
        it = 0
        for token in tokens:
            if token == ZERO_ADDRESS:
                break
            token.transfer(deployer, token_balances[it], {"from": accounts[0]})
            token.approve(swapfactory, token_balances[it], {"from": deployer})
            it += 1

        if template_index is None:
            template_index = 1 if amp != 2**64 else 0

        tx_R = swapfactory.deploy_swappool(
            template_index,
            tokens,
            token_balances,
            weights,
            amp,
            name,
            symbol,
            ZERO_ADDRESS if only_local else crosschaininterface,
            {"from": deployer},
        )

        if amp == 2**64:
            return CatalystSwapPool.at(tx_R.return_value)
        else:
            return CatalystSwapPoolAmplified.at(tx_R.return_value)

    yield deploy_swappool


@pytest.fixture(scope="module")
def channelId():
    yield convert.to_bytes(1, type_str="bytes32")


def para_tokens(provided, token_list):
    out_list = []
    for val in provided:
        out_list.append(token_list[val])
    return out_list


# Non amplified pools
@pytest.fixture(scope="module")
def swappool(channelId, deploy_swappool, pool_data, token_list):
    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = para_tokens(pool_data.get("tokens"), token_list)
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


@pytest.fixture(scope="module")
def swappool1(deploy_swappool, pool_data_cross, token_list):
    pool_data = pool_data_cross[0]
    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = para_tokens(pool_data.get("tokens"), token_list)
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

    yield sp
    
    
@pytest.fixture(scope="module")
def swappool2(deploy_swappool, pool_data_cross, token_list):
    pool_data = pool_data_cross[1]
    assert pool_data.get("amp") == 2**64
    deployer = pool_data.get("deployer")
    tokens = para_tokens(pool_data.get("tokens"), token_list)
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
    
    yield sp
    
    
# Amplified pool
@pytest.fixture(scope="module")
def swappool_amp(channelId, deploy_swappool, amp_pool_data, token_list):
    assert amp_pool_data.get("amp") < 2**64
    deployer = amp_pool_data.get("deployer")
    tokens = para_tokens(amp_pool_data.get("tokens"), token_list)
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


# Amplified pool
@pytest.fixture(scope="module")
def swappool1_amp(deploy_swappool, amp_pool_data_cross, token_list):
    amp_pool_data = amp_pool_data_cross[0]
    assert amp_pool_data.get("amp") < 2**64
    deployer = amp_pool_data.get("deployer")
    tokens = para_tokens(amp_pool_data.get("tokens"), token_list)
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

    yield sp


# Amplified pool
@pytest.fixture(scope="module")
def swappool2_amp(deploy_swappool, amp_pool_data_cross, token_list):
    amp_pool_data = amp_pool_data_cross[1]
    assert amp_pool_data.get("amp") < 2**64
    deployer = amp_pool_data.get("deployer")
    tokens = para_tokens(amp_pool_data.get("tokens"), token_list)
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

    yield sp