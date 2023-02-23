import pytest
from ape import reverts
from ape.utils import ZERO_ADDRESS

from fixtures.pools import MAX_POOL_ASSETS


@pytest.fixture(scope="module")
def swap_pool_template_idx(swap_pool_type, volatile_swap_pool_template, amplified_swap_pool_template):
    return volatile_swap_pool_template if swap_pool_type == "volatile" else amplified_swap_pool_template



# Main setup parametrized test **************************************************************************************************
# Test that all provided pool configs get deployed correctly
def test_setup(
    pool_config,
    pool_tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov
):

    for i, token in enumerate(pool_tokens):
        token.approve(swap_factory, pool_config["init_balances"][i], sender=gov)
    
    tx = swap_factory.deploy_swappool(
        swap_pool_template_idx,
        pool_tokens,
        pool_config["init_balances"],
        pool_config["weights"],
        amplification,
        0,
        pool_config["name"],
        pool_config["symbol"],
        ZERO_ADDRESS,
        sender=gov
    )

    # TODO verify that all parameters are saved correctly on-chain



# Tokens and weights *************************************************************************************************************

def test_setup_no_tokens(
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
):
    with reverts():      # ! Should be filtered with dev_revert_msg="dev: invalid asset count"
        swap_factory.deploy_swappool(
            swap_pool_template_idx,
            [],
            [],
            [],
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            sender=gov
        )


@pytest.mark.parametrize("asset_count", range(1, MAX_POOL_ASSETS+1))
def test_setup_valid_token_count(
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    asset_count
):

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)

    swap_factory.deploy_swappool(
        swap_pool_template_idx,
        tokens[:asset_count],
        [10**8]*asset_count,
        [1]*asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        sender=gov
    )


def test_setup_too_many_tokens(
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets + 1

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)

    with reverts():      # ! Should be filtered with dev_revert_msg="dev: invalid asset count"
        swap_factory.deploy_swappool(
            swap_pool_template_idx,
            tokens[:asset_count],
            [10**8]*asset_count,
            [1]*asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            sender=gov
        )


def test_setup_no_balance_set(
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)


    with reverts():      # ! Should be filtered with dev_revert_msg="dev: 0 tokens provided in setup."
        swap_factory.deploy_swappool(
            swap_pool_template_idx,
            tokens[:asset_count],
            [10**8]*(asset_count-1) + [0],  # ! Last balance argument set to 0
            [1]*asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            sender=gov
        )


def test_setup_no_weight_set(
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)


    with reverts():      # ! Should be filtered with dev_revert_msg="dev: invalid 0-valued weight provided"
        swap_factory.deploy_swappool(
            swap_pool_template_idx,
            tokens[:asset_count],
            [10**8]*asset_count,
            [1]*(asset_count-1) + [0],  # ! Last weight argument set to 0
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            sender=gov
        )


def test_setup_without_funds(
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets

    # ! Not approving tokens to the factory on purpose

    with reverts("TRANSFER_FROM_FAILED"):
        swap_factory.deploy_swappool(
            swap_pool_template_idx,
            tokens[:asset_count],
            [10**8]*asset_count,
            [1]*asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            sender=gov
        )



# Misc **************************************************************************************************************************

def test_setup_invalid_template(
    tokens,
    swap_factory,
    amplification,
    gov,
    max_pool_assets,
    swap_pool_type,
     volatile_swap_pool_template, 
     amplified_swap_pool_template
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)
    
    with reverts():      # ! Should be filtered with dev_revert_msg="dev: amplification not set correctly."
        swap_factory.deploy_swappool(
            amplified_swap_pool_template if swap_pool_type == "volatile" else volatile_swap_pool_template,          # ! Invalid template selected on purpose
            tokens[:asset_count],
            [10**8]*asset_count,
            [1]*asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            sender=gov
        )


def test_setup_pool_token_mint(
    project,
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)
    
    tx = swap_factory.deploy_swappool(
        swap_pool_template_idx,
        tokens[:asset_count],
        [10**8]*asset_count,
        [1]*asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        sender=gov
    )

    sp = project.CatalystSwapPoolVolatile.at(tx.return_value)

    # Verify pool tokens have been minted for the gov
    assert sp.balanceOf(gov) == 10**18



def test_setup_call_setup_external(
    project,
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)
    
    tx = swap_factory.deploy_swappool(
        swap_pool_template_idx,
        tokens[:asset_count],
        [10**8]*asset_count,
        [1]*asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        sender=gov
    )

    sp = project.CatalystSwapPoolVolatile.at(tx.return_value)

    # Call setup again
    with reverts():      # ! Should be filtered with dev_revert_msg="dev: Pool Already setup."
        sp.setup(
            "",
            "",
            ZERO_ADDRESS,
            0,
            0,
            ZERO_ADDRESS,
            gov,
            sender=gov
        )



def test_setup_call_initialize_swap_curves_external(
    project,
    tokens,
    swap_factory,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)
    
    tx = swap_factory.deploy_swappool(
        swap_pool_template_idx,
        tokens[:asset_count],
        [10**8]*asset_count,
        [1]*asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        sender=gov
    )

    sp = project.CatalystSwapPoolVolatile.at(tx.return_value)

    # Call initializeSwapCurves again
    with reverts():      # ! Should be filtered with dev_revert_msg="dev: swap curves may only be initialized once by the factory"
        sp.initializeSwapCurves(
            tokens[:asset_count],
            [1]*asset_count,
            10**18,
            gov,
            sender=gov
        )



@pytest.mark.parametrize("onlyLocal", [True, False])
def test_setup_only_local(
    project,
    tokens,
    swap_factory,
    cross_chain_interface,
    amplification,
    swap_pool_template_idx,
    gov,
    max_pool_assets,
    onlyLocal
):
    asset_count = max_pool_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8, sender=gov)
    
    tx = swap_factory.deploy_swappool(
        swap_pool_template_idx,
        tokens[:asset_count],
        [10**8]*asset_count,
        [1]*asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS if onlyLocal else cross_chain_interface,
        sender=gov
    )

    sp = project.CatalystSwapPoolVolatile.at(tx.return_value)

    assert sp.onlyLocal() == onlyLocal
