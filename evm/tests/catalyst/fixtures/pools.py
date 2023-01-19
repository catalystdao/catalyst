import pytest
from brownie import (
    convert,
    ZERO_ADDRESS,
    CatalystSwapPool,
    CatalystSwapPoolAmplified
)

import tests.catalyst.utils.pool_utils as pool_utils
from tests.catalyst.utils.pool_utils import compute_balance_0, compute_invariant

MAX_POOL_ASSETS = 3

@pytest.fixture(scope="module")
def deploy_pool(accounts, swap_factory, cross_chain_interface, swap_pool_type, deployer):
    def _deploy_pool(
        tokens,
        token_balances,
        weights,
        amp,
        name,
        symbol,
        deployer = deployer,
        only_local = False,
        template_index = None
    ):
        for i, token in enumerate(tokens):
            token.transfer(deployer, token_balances[i], {"from": accounts[0]})
            token.approve(swap_factory, token_balances[i], {"from": deployer})

        if template_index is None:
            if swap_pool_type == "volatile":
                template_index = 0
            elif swap_pool_type == "amplified":
                template_index = 1
            else:
                raise Exception(f"Unknown swap_pool_type \'{swap_pool_type}\'.")

        tx = swap_factory.deploy_swappool(
            template_index,
            tokens,
            token_balances,
            weights,
            amp,
            name,
            symbol,
            ZERO_ADDRESS if only_local else cross_chain_interface,
            {"from": deployer},
        )

        if template_index == 0:
            return CatalystSwapPool.at(tx.return_value)
        else:
            return CatalystSwapPoolAmplified.at(tx.return_value)

    yield _deploy_pool



@pytest.fixture(scope="module")
def max_pool_assets():
    return MAX_POOL_ASSETS


@pytest.fixture(scope="module")
def swap_pool_type(raw_config):
    yield raw_config["swap_pool_type"]


@pytest.fixture(scope="module")
def amplification(request, raw_config, swap_pool_type):

    if swap_pool_type == "volatile":
        yield None

    elif swap_pool_type == "amplified":

        # NOTE: the --amplification flag overrides the amplification value set on the config file if present
        amplification = request.config.getoption("--amplification") or raw_config["amplification"]
        amplification = eval(amplification)     # Parse expressions such as '10**18'

        assert amplification < 10**18 and amplification > 0

        yield amplification


@pytest.fixture(scope="module")
def channel_id():
    yield convert.to_bytes(1, type_str="bytes32")



# 'group_' fixtures
# Each of these expose info on ALL the pools defined on the loaded test config file

@pytest.fixture(scope="module")
def group_config(raw_config, amplification, max_pool_assets):

    raw_pools_config = raw_config["pools"]

    assert len(raw_pools_config) >= 1, "At least 1 pool must be defined on the test config file."


    # Verify the pools config
    for config in raw_pools_config:
        _verify_pool_config(config, max_pool_assets)

    # Inject the amplification value into each pool config object
    yield [
        {
            "tokens"        : config["tokens"],
            "init_balances" : [eval(balance) for balance in config["initBalances"]],      # Evaluate balance expressions (e.g. '10**18')
            "weights"       : config["weights"],
            "name"          : config["name"],
            "symbol"        : config["symbol"],
            "amplification" : amplification
        } for config in raw_pools_config
    ]


@pytest.fixture(scope="module")
def group_tokens(group_config, tokens):
    yield [[tokens[i] for i in pool["tokens"]] for pool in group_config]


@pytest.fixture(scope="module")
def group_pools(group_config, group_tokens, deploy_pool, deployer):

    yield [
        deploy_pool(
            tokens         = tokens,
            token_balances = pool["init_balances"],
            weights        = pool["weights"],
            amp            = pool["amplification"] if pool["amplification"] is not None else 10**18,
            name           = pool["name"],
            symbol         = pool["symbol"],
            deployer       = deployer,
        ) for pool, tokens in zip(group_config, group_tokens)
    ]



# Single pool parametrized fixtures

@pytest.fixture(scope="module")
def pool_index(raw_config):
    yield raw_config["pool_index"]

@pytest.fixture(scope="module")
def pool(group_pools, pool_index):
    yield group_pools[pool_index]

@pytest.fixture(scope="module")
def pool_config(group_config, pool_index):
    yield group_config[pool_index]

@pytest.fixture(scope="module")
def pool_tokens(group_tokens, pool_index):
    yield group_tokens[pool_index]



# Dual pool parametrized fixtures

@pytest.fixture(scope="module")
def source_pool_index(raw_config):
    yield raw_config["source_index"]

@pytest.fixture(scope="module")
def source_pool(group_pools, source_pool_index):
    yield group_pools[source_pool_index]

@pytest.fixture(scope="module")
def source_pool_config(group_config, source_pool_index):
    yield group_config[source_pool_index]

@pytest.fixture(scope="module")
def source_pool_tokens(group_tokens, source_pool_index):
    yield group_tokens[source_pool_index]


@pytest.fixture(scope="module")
def target_pool_index(raw_config):
    yield raw_config["target_index"]

@pytest.fixture(scope="module")
def target_pool(group_pools, target_pool_index):
    yield group_pools[target_pool_index]

@pytest.fixture(scope="module")
def target_pool_config(group_config, target_pool_index):
    yield group_config[target_pool_index]

@pytest.fixture(scope="module")
def target_pool_tokens(group_tokens, target_pool_index):
    yield group_tokens[target_pool_index]




def _verify_pool_config(config, max_pool_assets):
    assert "tokens" in config and len(config["tokens"]) > 0 and len(config["tokens"]) <= max_pool_assets
    assert "initBalances" in config and len(config["initBalances"]) == len(config["tokens"])
    assert "weights" in config and len(config["weights"]) == len(config["tokens"])
    assert "name" in config and isinstance(config["name"], str)
    assert "symbol" in config and isinstance(config["symbol"], str)




# Pool Modifiers ****************************************************************************************************************

@pytest.fixture(scope="module")
def group_finish_setup(group_pools, deployer):
    for pool in group_pools:
        pool.finishSetup({"from": deployer})

@pytest.fixture(scope="module")
def group_connect_pools(group_pools, channel_id, deployer):

    for source_pool in group_pools:
        for target_pool in group_pools:

            if source_pool == target_pool:
                continue
            
            source_pool.createConnection(
                channel_id,
                convert.to_bytes(target_pool.address.replace("0x", "")),
                True,
                {"from": deployer}
            )


@pytest.fixture(scope="module")
def pool_finish_setup(pool, deployer):
    pool.finishSetup({"from": deployer})

@pytest.fixture(scope="module")
def pool_connect_itself(pool, channel_id, deployer):
    pool.createConnection(
        channel_id,
        convert.to_bytes(pool.address.replace("0x", "")),
        True,
        {"from": deployer}
    )


# Pool Query and Calculations Helpers *******************************************************************************************

# Weights

@pytest.fixture(scope="module")
def get_pool_weights(pool, pool_tokens):
    def _get_pool_weights():
        return [pool._weight(token) for token in pool_tokens]
    
    yield _get_pool_weights

@pytest.fixture(scope="module")
def get_source_pool_weights(source_pool, source_pool_tokens):
    def _get_source_source_pool_weights():
        return [source_pool._weight(token) for token in source_pool_tokens]
    
    yield _get_source_source_pool_weights

@pytest.fixture(scope="module")
def get_target_pool_weights(target_pool, target_pool_tokens):
    def _get_target_pool_weights():
        return [target_pool._weight(token) for token in target_pool_tokens]
    
    yield _get_target_pool_weights



# Token Balances

@pytest.fixture(scope="module")
def get_pool_balances(pool, pool_tokens):
    def _get_pool_balances():
        return [token.balanceOf(pool) for token in pool_tokens]
    
    yield _get_pool_balances

@pytest.fixture(scope="module")
def get_source_pool_balances(source_pool, source_pool_tokens):
    def _get_source_source_pool_balances():
        return [token.balanceOf(source_pool) for token in source_pool_tokens]
    
    yield _get_source_source_pool_balances

@pytest.fixture(scope="module")
def get_target_pool_balances(target_pool, target_pool_tokens):
    def _get_target_pool_balances():
        return [token.balanceOf(target_pool) for token in target_pool_tokens]
    
    yield _get_target_pool_balances



# Amplification

@pytest.fixture(scope="module")
def get_pool_amp(pool):
    def _get_pool_amp():
        try:
            amp = pool._amp()   # Amplified pools
        except AttributeError:
            amp = 10**18        # Volatile pools

        return amp
    
    yield _get_pool_amp

@pytest.fixture(scope="module")
def get_source_pool_amp(source_pool):
    def _get_source_source_pool_amp():
        try:
            amp = source_pool._amp()   # Amplified pools
        except AttributeError:
            amp = 10**18               # Volatile pools

        return amp
    
    yield _get_source_source_pool_amp

@pytest.fixture(scope="module")
def get_target_pool_amp(target_pool):
    def _get_target_pool_amp():
        try:
            amp = target_pool._amp()   # Amplified pools
        except AttributeError:
            amp = 10**18               # Volatile pools

        return amp
    
    yield _get_target_pool_amp



# Unit Tracker

@pytest.fixture(scope="module")
def get_pool_unit_tracker(pool):
    def _get_pool_unit_tracker():
        return pool._unitTracker()
    
    yield _get_pool_unit_tracker

@pytest.fixture(scope="module")
def get_source_pool_unit_tracker(source_pool):
    def _get_source_source_pool_unit_tracker():
        return source_pool._unitTracker()
    
    yield _get_source_source_pool_unit_tracker

@pytest.fixture(scope="module")
def get_target_pool_unit_tracker(target_pool):
    def _get_target_pool_unit_tracker():
        return target_pool._unitTracker()
    
    yield _get_target_pool_unit_tracker



# Invariant

@pytest.fixture(scope="module")
def get_pool_invariant(get_pool_weights, get_pool_balances, get_pool_amp):
    def _get_pool_invariant():
        return compute_invariant(get_pool_weights(), get_pool_balances(), get_pool_amp())

    yield _get_pool_invariant

@pytest.fixture(scope="module")
def get_source_pool_invariant(get_source_pool_weights, get_source_pool_balances, get_source_pool_amp):
    def _get_source_pool_invariant():
        return compute_invariant(get_source_pool_weights(), get_source_pool_balances(), get_source_pool_amp())

    yield _get_source_pool_invariant

@pytest.fixture(scope="module")
def get_target_pool_invariant(get_target_pool_weights, get_target_pool_balances, get_target_pool_amp):
    def _get_target_pool_invariant():
        return compute_invariant(get_target_pool_weights(), get_target_pool_balances(), get_target_pool_amp())

    yield _get_target_pool_invariant



# Balance 0 (Only Amplified!)

@pytest.fixture(scope="module")
def get_pool_balance_0(get_pool_weights, get_pool_balances, get_pool_unit_tracker, get_pool_amp):
    def _get_pool_balance_0():
        return compute_balance_0(
            get_pool_weights(),
            get_pool_balances(),
            get_pool_unit_tracker(),
            get_pool_amp()
        )

    yield _get_pool_balance_0

@pytest.fixture(scope="module")
def get_source_pool_balance_0(
    get_source_pool_weights,
    get_source_pool_balances,
    get_source_pool_unit_tracker,
    get_source_pool_amp
):
    def _get_source_pool_balance_0():
        return compute_balance_0(
            get_source_pool_weights(),
            get_source_pool_balances(),
            get_source_pool_unit_tracker(),
            get_source_pool_amp()
        )

    yield _get_source_pool_balance_0

@pytest.fixture(scope="module")
def get_target_pool_balance_0(
    get_target_pool_weights,
    get_target_pool_balances,
    get_target_pool_unit_tracker,
    get_target_pool_amp
):
    def _get_target_pool_balance_0():
        return compute_balance_0(
            get_target_pool_weights(),
            get_target_pool_balances(),
            get_target_pool_unit_tracker(),
            get_target_pool_amp()
        )

    yield _get_target_pool_balance_0




# Swap Calculations Helpers *****************************************************************************************************

# NOTE: this fixture is only expected to be used on tests that include the fixture 'pool'
@pytest.fixture(scope="module")
def compute_expected_local_swap(
    pool,
    get_pool_amp
):
    def _compute_expected_local_swap(
        swap_amount,
        from_token,
        to_token
    ):
        pool_amp = get_pool_amp()

        return pool_utils.compute_expected_swap(
            swap_amount,
            pool._weight(from_token),
            from_token.balanceOf(pool),
            pool._weight(to_token),
            to_token.balanceOf(pool),
            pool_amp,
            pool_amp
        )['output']
    
    yield _compute_expected_local_swap


# NOTE: this fixture is only expected to be used on tests that include the fixtures 'source_pool' and 'target_pool'
@pytest.fixture(scope="module")
def compute_expected_swap(
    source_pool,
    target_pool,
    get_source_pool_amp,
    get_target_pool_amp
):
    def _compute_expected_swap(
        swap_amount,
        from_token,
        to_token
    ):
        return pool_utils.compute_expected_swap(
            swap_amount,
            source_pool._weight(from_token),
            from_token.balanceOf(source_pool),
            target_pool._weight(to_token),
            to_token.balanceOf(target_pool),
            get_source_pool_amp(),
            get_target_pool_amp()
        )
    
    yield _compute_expected_swap


# NOTE: this fixture is only expected to be used on tests that include the fixture 'target_pool'
@pytest.fixture(scope="module")
def compute_expected_swap_given_U(
    target_pool,
    get_target_pool_amp
):
    def _compute_expected_swap_given_U(
        U,
        to_token
    ):
        return pool_utils.compute_expected_swap_given_U(
            U,
            target_pool._weight(to_token),
            to_token.balanceOf(target_pool),
            get_target_pool_amp()
        )
    
    yield _compute_expected_swap_given_U