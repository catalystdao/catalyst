import pytest
import brownie


@pytest.fixture(scope="module")
def sample_vault(deploy_vault, tokens, deployer, amplification, max_vault_assets):
    yield deploy_vault(
        tokens=tokens[:max_vault_assets],
        token_balances=[10**8] * max_vault_assets,
        weights=[1] * max_vault_assets,
        amp=amplification,
        name="",
        symbol="",
        deployer=deployer,
    )


# Main setup parametrized test **************************************************************************************************
# Test that all provided vault configs work correctly
def test_finish_setup(vault, deployer):

    assert not vault.ready()

    vault.finishSetup({"from": deployer})

    assert vault.ready()

    # TODO verify that all parameters are saved correctly on-chain


# Authority and state tests *****************************************************************************************************


def test_finish_setup_unauthorized(sample_vault, molly):

    with brownie.reverts():
        sample_vault.finishSetup({"from": molly})


def test_finish_setup_twice(sample_vault, deployer):

    sample_vault.finishSetup({"from": deployer})

    with brownie.reverts():
        sample_vault.finishSetup({"from": deployer})


@pytest.mark.parametrize("onlyLocal", [True, False])
def test_finish_setup_only_local(
    deploy_vault, tokens, deployer, amplification, max_vault_assets, onlyLocal
):

    sp = deploy_vault(
        tokens=tokens[:max_vault_assets],
        token_balances=[10**8] * max_vault_assets,
        weights=[1] * max_vault_assets,
        amp=amplification,
        name="",
        symbol="",
        deployer=deployer,
        only_local=onlyLocal,
    )

    sp.finishSetup({"from": deployer})

    assert sp.onlyLocal() == onlyLocal


def test_finish_setup_event(sample_vault, deployer):

    tx = sample_vault.finishSetup({"from": deployer})

    assert "FinishSetup" in tx.events
