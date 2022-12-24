from brownie import CatalystSwapPool, CatalystSwapPoolAmplified, ZERO_ADDRESS

from .common import DEFAULT_POOL_DECIMALS, INITIAL_POOL_TOKEN_SUPPLY, SwapPoolInfo


def run_deploy_swappool_unsafe(
    tokens,
    balances,
    weights,
    amplification,
    name,
    symbol,
    swap_pool_factory,
    cross_chain_interface,
    deployer,
    gov,
    template_index = None
) -> SwapPoolInfo:

    init_deployer_balances = [token.balanceOf(deployer) for token in tokens]

    weights = weights or [1 for _ in tokens]
    template_index = template_index if template_index is not None else (1 if amplification != 2**64 else 0)


    # Make sure deployer has funds and approve allowance for the factory
    for i, (token, deploy_balance, deployer_balance) in enumerate(zip(tokens, balances, init_deployer_balances)):

        if deployer_balance < deploy_balance:
            if gov is None:
                raise Exception("Deployer does not have enough funds to deploy pool.")
            else:
                token.transfer(deployer, deploy_balance - deployer_balance, {"from": gov})
                init_deployer_balances[i] = deploy_balance
    
        token.approve(swap_pool_factory, deploy_balance, {"from": deployer})


    # Deploy
    tx = swap_pool_factory.deploy_swappool(
        template_index,
        tokens,
        balances,
        weights,
        amplification,
        name,
        symbol,
        cross_chain_interface,
        {"from": deployer}
    )


    # Verify pool state
    swap_pool = CatalystSwapPool.at(tx.return_value) if template_index == 0 else CatalystSwapPoolAmplified.at(tx.return_value)

    for i, (token, deploy_balance, init_deployer_balance) in enumerate(zip(tokens, balances, init_deployer_balances)):

        # Token and weight stored
        assert swap_pool._tokenIndexing(i) == token
        assert swap_pool._weight(token)    == weights[i]

        # Balance check
        assert token.balanceOf(swap_pool) == deploy_balance
        assert token.balanceOf(deployer)  == init_deployer_balance - deploy_balance

        # Balance0 check
        assert swap_pool._balance0(token) == deploy_balance

    # Pool tokens check
    assert swap_pool.totalSupply()       == INITIAL_POOL_TOKEN_SUPPLY
    assert swap_pool.balanceOf(deployer) == INITIAL_POOL_TOKEN_SUPPLY
    
    # Make sure pool is NOT in 'ready' state
    assert not swap_pool.ready()

    # Check other info
    assert swap_pool._factory()        == swap_pool_factory
    assert swap_pool._chaininterface() == cross_chain_interface
    assert swap_pool._setupMaster()    == deployer

    assert swap_pool.name()     == name
    assert swap_pool.symbol()   == symbol
    assert swap_pool.decimals() == DEFAULT_POOL_DECIMALS

    #TODO onlylocal
    #TODO governance fee
    #TODO feeAdministrator
    
    return SwapPoolInfo(swap_pool, tokens, balances, weights, amplification, 0, 0, name, symbol, deployer)


def run_deploy_swappool(
    tokens,
    balances,
    weights,
    name,
    symbol,
    swap_pool_factory,
    cross_chain_interface,
    deployer,
    gov,
    finish_setup = False
) -> SwapPoolInfo:

    swap_pool_info = run_deploy_swappool_unsafe(
        tokens                = tokens,
        balances              = balances,
        weights               = weights,
        amplification         = 2**64,
        name                  = name,
        symbol                = symbol,
        swap_pool_factory     = swap_pool_factory,
        cross_chain_interface = cross_chain_interface,
        deployer              = deployer,
        gov                   = gov,
        template_index        = 0
    )
    
    if finish_setup:
        run_finish_setup(swap_pool_info, deployer)
    
    return swap_pool_info


def run_amp_deploy_swappool(
    tokens,
    balances,
    weights,
    amplification,
    name,
    symbol,
    swap_pool_factory,
    cross_chain_interface,
    deployer,
    gov,
    finish_setup = False
) -> SwapPoolInfo:

    swap_pool_info = run_deploy_swappool_unsafe(
        tokens                = tokens,
        balances              = balances,
        weights               = weights,
        amplification         = amplification,
        name                  = name,
        symbol                = symbol,
        swap_pool_factory     = swap_pool_factory,
        cross_chain_interface = cross_chain_interface,
        deployer              = deployer,
        gov                   = gov,
        template_index        = 1
    )
    
    if finish_setup:
        run_amp_finish_setup(swap_pool_info, deployer)
    
    return swap_pool_info



def run_finish_setup(
    swap_pool_info: SwapPoolInfo,
    setup_master
):
    sp = swap_pool_info.swappool
    sp.finishSetup({"from": setup_master})
    
    assert sp._setupMaster() == ZERO_ADDRESS

    assert sp.ready()



def run_amp_finish_setup(
    swap_pool_info: SwapPoolInfo,
    setup_master
):
    run_finish_setup(swap_pool_info, setup_master)