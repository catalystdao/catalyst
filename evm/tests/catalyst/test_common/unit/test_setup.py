import pytest
import brownie
from brownie import ZERO_ADDRESS, CatalystVaultVolatile

from tests.catalyst.fixtures.vaults import MAX_VAULT_ASSETS


@pytest.fixture(scope="module")
def swap_vault_template_idx(
    swap_vault_type, volatile_swap_vault_template, amplified_swap_vault_template
):
    return (
        volatile_swap_vault_template
        if swap_vault_type == "volatile"
        else amplified_swap_vault_template
    )


# Main setup parametrized test **************************************************************************************************
# Test that all provided vault configs get deployed correctly
def test_setup(
    vault_config,
    vault_tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
):

    for i, token in enumerate(vault_tokens):
        token.approve(swap_factory, vault_config["init_balances"][i])

    tx = swap_factory.deploy_swapvault(
        swap_vault_template_idx,
        vault_tokens,
        vault_config["init_balances"],
        vault_config["weights"],
        amplification,
        0,
        vault_config["name"],
        vault_config["symbol"],
        ZERO_ADDRESS,
        {"from": deployer},
    )

    # TODO verify that all parameters are saved correctly on-chain


# Tokens and weights *************************************************************************************************************


def test_setup_no_tokens(
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
):
    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: invalid asset count"
        swap_factory.deploy_swapvault(
            swap_vault_template_idx,
            [],
            [],
            [],
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            {"from": deployer},
        )


@pytest.mark.parametrize("asset_count", range(1, MAX_VAULT_ASSETS + 1))
def test_setup_valid_token_count(
    tokens, swap_factory, amplification, swap_vault_template_idx, deployer, asset_count
):

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    swap_factory.deploy_swapvault(
        swap_vault_template_idx,
        tokens[:asset_count],
        [10**8] * asset_count,
        [1] * asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        {"from": deployer},
    )


def test_setup_too_many_tokens(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets + 1

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: invalid asset count"
        swap_factory.deploy_swapvault(
            swap_vault_template_idx,
            tokens[:asset_count],
            [10**8] * asset_count,
            [1] * asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            {"from": deployer},
        )


def test_setup_no_balance_set(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: 0 tokens provided in setup."
        swap_factory.deploy_swapvault(
            swap_vault_template_idx,
            tokens[:asset_count],
            [10**8] * (asset_count - 1) + [0],  # ! Last balance argument set to 0
            [1] * asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            {"from": deployer},
        )


def test_setup_no_weight_set(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: invalid 0-valued weight provided"
        swap_factory.deploy_swapvault(
            swap_vault_template_idx,
            tokens[:asset_count],
            [10**8] * asset_count,
            [1] * (asset_count - 1) + [0],  # ! Last weight argument set to 0
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            {"from": deployer},
        )


def test_setup_without_funds(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets

    # ! Not approving tokens to the factory on purpose

    with brownie.reverts("TRANSFER_FROM_FAILED"):
        swap_factory.deploy_swapvault(
            swap_vault_template_idx,
            tokens[:asset_count],
            [10**8] * asset_count,
            [1] * asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            {"from": deployer},
        )


# Misc **************************************************************************************************************************


def test_setup_invalid_template(
    tokens,
    swap_factory,
    amplification,
    deployer,
    max_vault_assets,
    swap_vault_type,
    volatile_swap_vault_template,
    amplified_swap_vault_template,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: amplification not set correctly."
        swap_factory.deploy_swapvault(
            amplified_swap_vault_template
            if swap_vault_type == "volatile"
            else volatile_swap_vault_template,  # ! Invalid template selected on purpose
            tokens[:asset_count],
            [10**8] * asset_count,
            [1] * asset_count,
            amplification,
            0,
            "",
            "",
            ZERO_ADDRESS,
            {"from": deployer},
        )


def test_setup_vault_token_mint(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    tx = swap_factory.deploy_swapvault(
        swap_vault_template_idx,
        tokens[:asset_count],
        [10**8] * asset_count,
        [1] * asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        {"from": deployer},
    )

    sp = CatalystVaultVolatile.at(tx.return_value)

    # Verify vault tokens have been minted for the deployer
    assert sp.balanceOf(deployer) == 10**18


def test_setup_call_setup_external(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    tx = swap_factory.deploy_swapvault(
        swap_vault_template_idx,
        tokens[:asset_count],
        [10**8] * asset_count,
        [1] * asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        {"from": deployer},
    )

    sp = CatalystVaultVolatile.at(tx.return_value)

    # Call setup again
    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: Vault Already setup."
        sp.setup("", "", ZERO_ADDRESS, 0, 0, ZERO_ADDRESS, deployer, {"from": deployer})


def test_setup_call_initialize_swap_curves_external(
    tokens,
    swap_factory,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    tx = swap_factory.deploy_swapvault(
        swap_vault_template_idx,
        tokens[:asset_count],
        [10**8] * asset_count,
        [1] * asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS,
        {"from": deployer},
    )

    sp = CatalystVaultVolatile.at(tx.return_value)

    # Call initializeSwapCurves again
    with brownie.reverts():  # ! Should be filtered with dev_revert_msg="dev: swap curves may only be initialized once by the factory"
        sp.initializeSwapCurves(
            tokens[:asset_count],
            [1] * asset_count,
            10**18,
            deployer,
            {"from": deployer},
        )


@pytest.mark.parametrize("onlyLocal", [True, False])
def test_setup_only_local(
    tokens,
    swap_factory,
    cross_chain_interface,
    amplification,
    swap_vault_template_idx,
    deployer,
    max_vault_assets,
    onlyLocal,
):
    asset_count = max_vault_assets

    for token in tokens[:asset_count]:
        token.approve(swap_factory, 10**8)

    tx = swap_factory.deploy_swapvault(
        swap_vault_template_idx,
        tokens[:asset_count],
        [10**8] * asset_count,
        [1] * asset_count,
        amplification,
        0,
        "",
        "",
        ZERO_ADDRESS if onlyLocal else cross_chain_interface,
        {"from": deployer},
    )

    sp = CatalystVaultVolatile.at(tx.return_value)

    assert sp.onlyLocal() == onlyLocal
