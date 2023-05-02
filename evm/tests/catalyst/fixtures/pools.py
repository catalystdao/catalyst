import pytest
from brownie import (
    convert,
    ZERO_ADDRESS,
    CatalystSwapPoolVolatile,
    CatalystSwapPoolAmplified
)

import tests.catalyst.utils.pool_utils as pool_utils
from tests.catalyst.utils.common_utils import convert_64_bytes_address
from tests.catalyst.utils.pool_utils import compute_balance_0, compute_invariant

from tests.catalyst.utils.pool_utils import compute_expected_max_unit_inflow

MAX_POOL_ASSETS = 3

@pytest.fixture(scope="module")
def deploy_pool(accounts, swap_factory, volatile_swap_pool_template, amplified_swap_pool_template,  cross_chain_interface, swap_pool_type, deployer):
    def _deploy_pool(
        tokens,
        token_balances,
        weights,
        amp,
        name,
        symbol,
        deployer = deployer,
        only_local = False,
        template_address = None
    ):
        for i, token in enumerate(tokens):
            token.transfer(deployer, token_balances[i], {"from": accounts[0]})
            token.approve(swap_factory, token_balances[i], {"from": deployer})

        if template_address is None:
            if swap_pool_type == "volatile":
                template_address = volatile_swap_pool_template.address
            elif swap_pool_type == "amplified":
                template_address = amplified_swap_pool_template.address
            else:
                raise Exception(f"Unknown swap_pool_type \'{swap_pool_type}\'.")

        tx = swap_factory.deploy_swappool(
            template_address,
            tokens,
            token_balances,
            weights,
            amp,
            0,  # pool fee
            name,
            symbol,
            ZERO_ADDRESS if only_local else cross_chain_interface,
            {"from": deployer},
        )

        if template_address == volatile_swap_pool_template.address:
            return CatalystSwapPoolVolatile.at(tx.return_value)
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
        yield 10**18

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
def group_config(raw_config, amplification):

    # Inject the amplification value into each pool config object
    yield [
        {
            "tokens"        : config["tokens"],
            "init_balances" : [eval(balance) for balance in config["initBalances"]],      # Evaluate balance expressions (e.g. '10**18')
            "weights"       : config["weights"],
            "name"          : config["name"],
            "symbol"        : config["symbol"],
            "amplification" : amplification
        } for config in raw_config["pools"]
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
            amp            = pool["amplification"],
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
def pool_1_index(raw_config):
    yield raw_config["pool_1_index"]

@pytest.fixture(scope="module")
def pool_1(group_pools, pool_1_index):
    yield group_pools[pool_1_index]

@pytest.fixture(scope="module")
def pool_1_config(group_config, pool_1_index):
    yield group_config[pool_1_index]

@pytest.fixture(scope="module")
def pool_1_tokens(group_tokens, pool_1_index):
    yield group_tokens[pool_1_index]


@pytest.fixture(scope="module")
def pool_2_index(raw_config):
    yield raw_config["pool_2_index"]

@pytest.fixture(scope="module")
def pool_2(group_pools, pool_2_index):
    yield group_pools[pool_2_index]

@pytest.fixture(scope="module")
def pool_2_config(group_config, pool_2_index):
    yield group_config[pool_2_index]

@pytest.fixture(scope="module")
def pool_2_tokens(group_tokens, pool_2_index):
    yield group_tokens[pool_2_index]




# Pool Modifiers ****************************************************************************************************************

@pytest.fixture(scope="module")
def group_finish_setup(group_pools, deployer):
    for pool in group_pools:
        pool.finishSetup({"from": deployer})

@pytest.fixture(scope="module")
def group_connect_pools(group_pools, channel_id, deployer):

    for pool_1 in group_pools:
        for pool_2 in group_pools:

            if pool_1 == pool_2:
                continue
            
            pool_1.setConnection(
                channel_id,
                convert_64_bytes_address(pool_2.address),
                True,
                {"from": deployer}
            )


@pytest.fixture(scope="module")
def pool_finish_setup(pool, deployer):
    pool.finishSetup({"from": deployer})

@pytest.fixture(scope="module")
def pool_connect_itself(pool, channel_id, deployer):
    pool.setConnection(
        channel_id,
        convert_64_bytes_address(pool.address),
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
def get_pool_1_weights(pool_1, pool_1_tokens):
    def _get_pool_1_weights():
        return [pool_1._weight(token) for token in pool_1_tokens]
    
    yield _get_pool_1_weights

@pytest.fixture(scope="module")
def get_pool_2_weights(pool_2, pool_2_tokens):
    def _get_pool_2_weights():
        return [pool_2._weight(token) for token in pool_2_tokens]
    
    yield _get_pool_2_weights



# Token Balances

@pytest.fixture(scope="module")
def get_pool_balances(pool, pool_tokens):
    def _get_pool_balances():
        return [token.balanceOf(pool) for token in pool_tokens]
    
    yield _get_pool_balances

@pytest.fixture(scope="module")
def get_pool_1_balances(pool_1, pool_1_tokens):
    def _get_pool_1_balances():
        return [token.balanceOf(pool_1) for token in pool_1_tokens]
    
    yield _get_pool_1_balances

@pytest.fixture(scope="module")
def get_pool_2_balances(pool_2, pool_2_tokens):
    def _get_pool_2_balances():
        return [token.balanceOf(pool_2) for token in pool_2_tokens]
    
    yield _get_pool_2_balances



# Amplification

@pytest.fixture(scope="module")
def get_pool_amp(pool):
    def _get_pool_amp():
        try:
            amp = 10**18 - pool._oneMinusAmp()   # Amplified pools
        except AttributeError:
            amp = 10**18        # Volatile pools

        return amp
    
    yield _get_pool_amp

@pytest.fixture(scope="module")
def get_pool_1_amp(pool_1):
    def _get_pool_1_amp():
        try:
            amp = 10**18 - pool_1._oneMinusAmp()   # Amplified pools
        except AttributeError:
            amp = 10**18               # Volatile pools

        return amp
    
    yield _get_pool_1_amp

@pytest.fixture(scope="module")
def get_pool_2_amp(pool_2):
    def _get_pool_2_amp():
        try:
            amp = 10**18 - pool_2._oneMinusAmp()   # Amplified pools
        except AttributeError:
            amp = 10**18               # Volatile pools

        return amp
    
    yield _get_pool_2_amp



# Unit Tracker

@pytest.fixture(scope="module")
def get_pool_unit_tracker(pool):
    def _get_pool_unit_tracker():
        return pool._unitTracker()
    
    yield _get_pool_unit_tracker

@pytest.fixture(scope="module")
def get_pool_1_unit_tracker(pool_1, get_pool_1_amp):
    def _get_pool_1_unit_tracker():
        if get_pool_1_amp() == 10**18:
            return 0
        return pool_1._unitTracker()
    
    yield _get_pool_1_unit_tracker

@pytest.fixture(scope="module")
def get_pool_2_unit_tracker(pool_2, get_pool_2_amp):
    def _get_pool_2_unit_tracker():
        if get_pool_2_amp() == 10**18:
            return 0
        return pool_2._unitTracker()
    
    yield _get_pool_2_unit_tracker



# Invariant

@pytest.fixture(scope="module")
def get_pool_invariant(get_pool_weights, get_pool_balances, get_pool_amp):
    def _get_pool_invariant():
        return compute_invariant(get_pool_weights(), get_pool_balances(), get_pool_amp())

    yield _get_pool_invariant

@pytest.fixture(scope="module")
def get_pool_1_invariant(get_pool_1_weights, get_pool_1_balances, get_pool_1_amp):
    def _get_pool_1_invariant():
        return compute_invariant(get_pool_1_weights(), get_pool_1_balances(), get_pool_1_amp())

    yield _get_pool_1_invariant

@pytest.fixture(scope="module")
def get_pool_2_invariant(get_pool_2_weights, get_pool_2_balances, get_pool_2_amp):
    def _get_pool_2_invariant():
        return compute_invariant(get_pool_2_weights(), get_pool_2_balances(), get_pool_2_amp())

    yield _get_pool_2_invariant



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
def get_pool_1_balance_0(
    get_pool_1_weights,
    get_pool_1_balances,
    get_pool_1_unit_tracker,
    get_pool_1_amp
):
    def _get_pool_1_balance_0():
        return compute_balance_0(
            get_pool_1_weights(),
            get_pool_1_balances(),
            get_pool_1_unit_tracker(),
            get_pool_1_amp()
        )

    yield _get_pool_1_balance_0

@pytest.fixture(scope="module")
def get_pool_2_balance_0(
    get_pool_2_weights,
    get_pool_2_balances,
    get_pool_2_unit_tracker,
    get_pool_2_amp
):
    def _get_pool_2_balance_0():
        return compute_balance_0(
            get_pool_2_weights(),
            get_pool_2_balances(),
            get_pool_2_unit_tracker(),
            get_pool_2_amp()
        )

    yield _get_pool_2_balance_0



# Max unit inflow

@pytest.fixture(scope="module")
def get_pool_max_unit_inflow(
    get_pool_weights,
    get_pool_balances,
    get_pool_amp
):
    def _get_pool_max_unit_inflow():
        return compute_expected_max_unit_inflow(
            get_pool_weights(),
            get_pool_balances(),
            get_pool_amp()
        )

    yield _get_pool_max_unit_inflow

@pytest.fixture(scope="module")
def get_pool_1_max_unit_inflow(
    get_pool_1_weights,
    get_pool_1_balances,
    get_pool_1_amp
):
    def _get_pool_1_max_unit_inflow():
        return compute_expected_max_unit_inflow(
            get_pool_1_weights(),
            get_pool_1_balances(),
            get_pool_1_amp()
        )

    yield _get_pool_1_max_unit_inflow

@pytest.fixture(scope="module")
def get_pool_2_max_unit_inflow(
    get_pool_2_weights,
    get_pool_2_balances,
    get_pool_2_amp
):
    def _get_pool_2_max_unit_inflow():
        return compute_expected_max_unit_inflow(
            get_pool_2_weights(),
            get_pool_2_balances(),
            get_pool_2_amp()
        )

    yield _get_pool_2_max_unit_inflow


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
            pool_amp,
            pool._poolFee() / 10**18,
            pool._governanceFeeShare() / 10**18
        )
    
    yield _compute_expected_local_swap


# NOTE: this fixture is only expected to be used on tests that include the fixtures 'pool_1' and 'pool_2'
@pytest.fixture(scope="module")
def compute_expected_swap(
    pool_1,
    pool_2,
    get_pool_1_amp,
    get_pool_2_amp
):
    def _compute_expected_swap(
        swap_amount,
        from_token,
        to_token
    ):
        return pool_utils.compute_expected_swap(
            swap_amount,
            pool_1._weight(from_token),
            from_token.balanceOf(pool_1),
            pool_2._weight(to_token),
            to_token.balanceOf(pool_2),
            get_pool_1_amp(),
            get_pool_2_amp(),
            pool_1._poolFee() / 10**18,
            pool_1._governanceFeeShare() / 10**18
        )
    
    yield _compute_expected_swap


# NOTE: this fixture is only expected to be used on tests that include the fixtures 'pool_1' and 'pool_2'
@pytest.fixture(scope="module")
def compute_expected_liquidity_swap(
    pool_1,
    pool_2,
    get_pool_1_weights,
    get_pool_1_balances,
    get_pool_1_unit_tracker,
    get_pool_2_weights,
    get_pool_2_balances,
    get_pool_2_unit_tracker,
    get_pool_1_amp,
    get_pool_2_amp
):
    def _compute_expected_liquidity_swap(
        swap_amount
    ):
        return pool_utils.compute_expected_liquidity_swap(
            swap_amount,
            get_pool_1_weights(),
            get_pool_1_balances(),
            pool_1.totalSupply(),
            get_pool_1_unit_tracker(),
            get_pool_2_weights(),
            get_pool_2_balances(),
            pool_2.totalSupply(),
            get_pool_2_unit_tracker(),
            get_pool_1_amp(),
            get_pool_2_amp()
        )
    
    yield _compute_expected_liquidity_swap



# NOTE: this fixture is only expected to be used on tests that include the fixture 'pool_2'
@pytest.fixture(scope="module")
def compute_expected_swap_given_U(
    pool_2,
    get_pool_2_amp
):
    def _compute_expected_swap_given_U(
        U,
        to_token
    ):
        return pool_utils.compute_expected_swap_given_U(
            U,
            pool_2._weight(to_token),
            to_token.balanceOf(pool_2),
            get_pool_2_amp()
        )
    
    yield _compute_expected_swap_given_U