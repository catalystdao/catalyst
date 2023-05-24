import pytest
from brownie import (
    convert,
    ZERO_ADDRESS,
    CatalystVaultVolatile,
    CatalystVaultAmplified,
)

import tests.catalyst.utils.vault_utils as vault_utils
from tests.catalyst.utils.vault_utils import compute_balance_0, compute_invariant

from tests.catalyst.utils.vault_utils import compute_expected_max_unit_inflow

MAX_VAULT_ASSETS = 3


@pytest.fixture(scope="module")
def deploy_vault(
    accounts,
    swap_factory,
    volatile_swap_vault_template,
    amplified_swap_vault_template,
    cross_chain_interface,
    swap_vault_type,
    deployer,
):
    def _deploy_vault(
        tokens,
        token_balances,
        weights,
        amp,
        name,
        symbol,
        deployer=deployer,
        only_local=False,
        template_address=None,
    ):
        for i, token in enumerate(tokens):
            token.transfer(deployer, token_balances[i], {"from": accounts[0]})
            token.approve(swap_factory, token_balances[i], {"from": deployer})

        if template_address is None:
            if swap_vault_type == "volatile":
                template_address = volatile_swap_vault_template.address
            elif swap_vault_type == "amplified":
                template_address = amplified_swap_vault_template.address
            else:
                raise Exception(f"Unknown swap_vault_type '{swap_vault_type}'.")

        tx = swap_factory.deploy_swapvault(
            template_address,
            tokens,
            token_balances,
            weights,
            amp,
            0,  # vault fee
            name,
            symbol,
            ZERO_ADDRESS if only_local else cross_chain_interface,
            {"from": deployer},
        )

        if template_address == volatile_swap_vault_template.address:
            return CatalystVaultVolatile.at(tx.return_value)
        else:
            return CatalystVaultAmplified.at(tx.return_value)

    yield _deploy_vault


@pytest.fixture(scope="module")
def max_vault_assets():
    return MAX_VAULT_ASSETS


@pytest.fixture(scope="module")
def swap_vault_type(raw_config):
    yield raw_config["swap_vault_type"]


@pytest.fixture(scope="module")
def amplification(request, raw_config, swap_vault_type):

    if swap_vault_type == "volatile":
        yield 10**18

    elif swap_vault_type == "amplified":

        # NOTE: the --amplification flag overrides the amplification value set on the config file if present
        amplification = (
            request.config.getoption("--amplification") or raw_config["amplification"]
        )
        amplification = eval(amplification)  # Parse expressions such as '10**18'

        assert amplification < 10**18 and amplification > 0

        yield amplification


@pytest.fixture(scope="module")
def channel_id():
    yield convert.to_bytes(1, type_str="bytes32")


# 'group_' fixtures
# Each of these expose info on ALL the vaults defined on the loaded test config file


@pytest.fixture(scope="module")
def group_config(raw_config, amplification):

    # Inject the amplification value into each vault config object
    yield [
        {
            "tokens": config["tokens"],
            "init_balances": [
                eval(balance) for balance in config["initBalances"]
            ],  # Evaluate balance expressions (e.g. '10**18')
            "weights": config["weights"],
            "name": config["name"],
            "symbol": config["symbol"],
            "amplification": amplification,
        }
        for config in raw_config["vaults"]
    ]


@pytest.fixture(scope="module")
def group_tokens(group_config, tokens):
    yield [[tokens[i] for i in vault["tokens"]] for vault in group_config]


@pytest.fixture(scope="module")
def group_vaults(group_config, group_tokens, deploy_vault, deployer):

    yield [
        deploy_vault(
            tokens=tokens,
            token_balances=vault["init_balances"],
            weights=vault["weights"],
            amp=vault["amplification"],
            name=vault["name"],
            symbol=vault["symbol"],
            deployer=deployer,
        )
        for vault, tokens in zip(group_config, group_tokens)
    ]


# Single vault parametrized fixtures


@pytest.fixture(scope="module")
def vault_index(raw_config):
    yield raw_config["vault_index"]


@pytest.fixture(scope="module")
def vault(group_vaults, vault_index):
    yield group_vaults[vault_index]


@pytest.fixture(scope="module")
def vault_config(group_config, vault_index):
    yield group_config[vault_index]


@pytest.fixture(scope="module")
def vault_tokens(group_tokens, vault_index):
    yield group_tokens[vault_index]


# Dual vault parametrized fixtures


@pytest.fixture(scope="module")
def vault_1_index(raw_config):
    yield raw_config["vault_1_index"]


@pytest.fixture(scope="module")
def vault_1(group_vaults, vault_1_index):
    yield group_vaults[vault_1_index]


@pytest.fixture(scope="module")
def vault_1_config(group_config, vault_1_index):
    yield group_config[vault_1_index]


@pytest.fixture(scope="module")
def vault_1_tokens(group_tokens, vault_1_index):
    yield group_tokens[vault_1_index]


@pytest.fixture(scope="module")
def vault_2_index(raw_config):
    yield raw_config["vault_2_index"]


@pytest.fixture(scope="module")
def vault_2(group_vaults, vault_2_index):
    yield group_vaults[vault_2_index]


@pytest.fixture(scope="module")
def vault_2_config(group_config, vault_2_index):
    yield group_config[vault_2_index]


@pytest.fixture(scope="module")
def vault_2_tokens(group_tokens, vault_2_index):
    yield group_tokens[vault_2_index]


# Vault Modifiers ****************************************************************************************************************


@pytest.fixture(scope="module")
def group_finish_setup(group_vaults, deployer):
    for vault in group_vaults:
        vault.finishSetup({"from": deployer})


@pytest.fixture(scope="module")
def group_connect_vaults(group_vaults, channel_id, deployer):

    for vault_1 in group_vaults:
        for vault_2 in group_vaults:

            if vault_1 == vault_2:
                continue

            vault_1.setConnection(
                channel_id,
                convert.to_bytes(20, "bytes1")
                + convert.to_bytes(0)
                + convert.to_bytes(vault_2.address.replace("0x", "")),
                True,
                {"from": deployer},
            )


@pytest.fixture(scope="module")
def vault_finish_setup(vault, deployer):
    vault.finishSetup({"from": deployer})


@pytest.fixture(scope="module")
def vault_connect_itself(vault, channel_id, deployer):
    vault.setConnection(
        channel_id,
        convert.to_bytes(20, "bytes1")
        + convert.to_bytes(0)
        + convert.to_bytes(vault.address.replace("0x", "")),
        True,
        {"from": deployer},
    )


# Vault Query and Calculations Helpers *******************************************************************************************

# Weights


@pytest.fixture(scope="module")
def get_vault_weights(vault, vault_tokens):
    def _get_vault_weights():
        return [vault._weight(token) for token in vault_tokens]

    yield _get_vault_weights


@pytest.fixture(scope="module")
def get_vault_1_weights(vault_1, vault_1_tokens):
    def _get_vault_1_weights():
        return [vault_1._weight(token) for token in vault_1_tokens]

    yield _get_vault_1_weights


@pytest.fixture(scope="module")
def get_vault_2_weights(vault_2, vault_2_tokens):
    def _get_vault_2_weights():
        return [vault_2._weight(token) for token in vault_2_tokens]

    yield _get_vault_2_weights


# Token Balances


@pytest.fixture(scope="module")
def get_vault_balances(vault, vault_tokens):
    def _get_vault_balances():
        return [token.balanceOf(vault) for token in vault_tokens]

    yield _get_vault_balances


@pytest.fixture(scope="module")
def get_vault_1_balances(vault_1, vault_1_tokens):
    def _get_vault_1_balances():
        return [token.balanceOf(vault_1) for token in vault_1_tokens]

    yield _get_vault_1_balances


@pytest.fixture(scope="module")
def get_vault_2_balances(vault_2, vault_2_tokens):
    def _get_vault_2_balances():
        return [token.balanceOf(vault_2) for token in vault_2_tokens]

    yield _get_vault_2_balances


# Amplification


@pytest.fixture(scope="module")
def get_vault_amp(vault):
    def _get_vault_amp():
        try:
            amp = 10**18 - vault._oneMinusAmp()  # Amplified vaults
        except AttributeError:
            amp = 10**18  # Volatile vaults

        return amp

    yield _get_vault_amp


@pytest.fixture(scope="module")
def get_vault_1_amp(vault_1):
    def _get_vault_1_amp():
        try:
            amp = 10**18 - vault_1._oneMinusAmp()  # Amplified vaults
        except AttributeError:
            amp = 10**18  # Volatile vaults

        return amp

    yield _get_vault_1_amp


@pytest.fixture(scope="module")
def get_vault_2_amp(vault_2):
    def _get_vault_2_amp():
        try:
            amp = 10**18 - vault_2._oneMinusAmp()  # Amplified vaults
        except AttributeError:
            amp = 10**18  # Volatile vaults

        return amp

    yield _get_vault_2_amp


# Unit Tracker


@pytest.fixture(scope="module")
def get_vault_unit_tracker(vault):
    def _get_vault_unit_tracker():
        return vault._unitTracker()

    yield _get_vault_unit_tracker


@pytest.fixture(scope="module")
def get_vault_1_unit_tracker(vault_1, get_vault_1_amp):
    def _get_vault_1_unit_tracker():
        if get_vault_1_amp() == 10**18:
            return 0
        return vault_1._unitTracker()

    yield _get_vault_1_unit_tracker


@pytest.fixture(scope="module")
def get_vault_2_unit_tracker(vault_2, get_vault_2_amp):
    def _get_vault_2_unit_tracker():
        if get_vault_2_amp() == 10**18:
            return 0
        return vault_2._unitTracker()

    yield _get_vault_2_unit_tracker


# Invariant


@pytest.fixture(scope="module")
def get_vault_invariant(get_vault_weights, get_vault_balances, get_vault_amp):
    def _get_vault_invariant():
        return compute_invariant(
            get_vault_weights(), get_vault_balances(), get_vault_amp()
        )

    yield _get_vault_invariant


@pytest.fixture(scope="module")
def get_vault_1_invariant(get_vault_1_weights, get_vault_1_balances, get_vault_1_amp):
    def _get_vault_1_invariant():
        return compute_invariant(
            get_vault_1_weights(), get_vault_1_balances(), get_vault_1_amp()
        )

    yield _get_vault_1_invariant


@pytest.fixture(scope="module")
def get_vault_2_invariant(get_vault_2_weights, get_vault_2_balances, get_vault_2_amp):
    def _get_vault_2_invariant():
        return compute_invariant(
            get_vault_2_weights(), get_vault_2_balances(), get_vault_2_amp()
        )

    yield _get_vault_2_invariant


# Balance 0 (Only Amplified!)


@pytest.fixture(scope="module")
def get_vault_balance_0(
    get_vault_weights, get_vault_balances, get_vault_unit_tracker, get_vault_amp
):
    def _get_vault_balance_0():
        return compute_balance_0(
            get_vault_weights(),
            get_vault_balances(),
            get_vault_unit_tracker(),
            get_vault_amp(),
        )

    yield _get_vault_balance_0


@pytest.fixture(scope="module")
def get_vault_1_balance_0(
    get_vault_1_weights, get_vault_1_balances, get_vault_1_unit_tracker, get_vault_1_amp
):
    def _get_vault_1_balance_0():
        return compute_balance_0(
            get_vault_1_weights(),
            get_vault_1_balances(),
            get_vault_1_unit_tracker(),
            get_vault_1_amp(),
        )

    yield _get_vault_1_balance_0


@pytest.fixture(scope="module")
def get_vault_2_balance_0(
    get_vault_2_weights, get_vault_2_balances, get_vault_2_unit_tracker, get_vault_2_amp
):
    def _get_vault_2_balance_0():
        return compute_balance_0(
            get_vault_2_weights(),
            get_vault_2_balances(),
            get_vault_2_unit_tracker(),
            get_vault_2_amp(),
        )

    yield _get_vault_2_balance_0


# Max unit inflow


@pytest.fixture(scope="module")
def get_vault_max_unit_inflow(get_vault_weights, get_vault_balances, get_vault_amp):
    def _get_vault_max_unit_inflow():
        return compute_expected_max_unit_inflow(
            get_vault_weights(), get_vault_balances(), get_vault_amp()
        )

    yield _get_vault_max_unit_inflow


@pytest.fixture(scope="module")
def get_vault_1_max_unit_inflow(
    get_vault_1_weights, get_vault_1_balances, get_vault_1_amp
):
    def _get_vault_1_max_unit_inflow():
        return compute_expected_max_unit_inflow(
            get_vault_1_weights(), get_vault_1_balances(), get_vault_1_amp()
        )

    yield _get_vault_1_max_unit_inflow


@pytest.fixture(scope="module")
def get_vault_2_max_unit_inflow(
    get_vault_2_weights, get_vault_2_balances, get_vault_2_amp
):
    def _get_vault_2_max_unit_inflow():
        return compute_expected_max_unit_inflow(
            get_vault_2_weights(), get_vault_2_balances(), get_vault_2_amp()
        )

    yield _get_vault_2_max_unit_inflow


# Swap Calculations Helpers *****************************************************************************************************

# NOTE: this fixture is only expected to be used on tests that include the fixture 'vault'
@pytest.fixture(scope="module")
def compute_expected_local_swap(vault, get_vault_amp):
    def _compute_expected_local_swap(swap_amount, from_token, to_token):
        vault_amp = get_vault_amp()

        return vault_utils.compute_expected_swap(
            swap_amount,
            vault._weight(from_token),
            from_token.balanceOf(vault),
            vault._weight(to_token),
            to_token.balanceOf(vault),
            vault_amp,
            vault_amp,
            vault._vaultFee() / 10**18,
            vault._governanceFeeShare() / 10**18,
        )

    yield _compute_expected_local_swap


# NOTE: this fixture is only expected to be used on tests that include the fixtures 'vault_1' and 'vault_2'
@pytest.fixture(scope="module")
def compute_expected_swap(vault_1, vault_2, get_vault_1_amp, get_vault_2_amp):
    def _compute_expected_swap(swap_amount, from_token, to_token):
        return vault_utils.compute_expected_swap(
            swap_amount,
            vault_1._weight(from_token),
            from_token.balanceOf(vault_1),
            vault_2._weight(to_token),
            to_token.balanceOf(vault_2),
            get_vault_1_amp(),
            get_vault_2_amp(),
            vault_1._vaultFee() / 10**18,
            vault_1._governanceFeeShare() / 10**18,
        )

    yield _compute_expected_swap


# NOTE: this fixture is only expected to be used on tests that include the fixtures 'vault_1' and 'vault_2'
@pytest.fixture(scope="module")
def compute_expected_liquidity_swap(
    vault_1,
    vault_2,
    get_vault_1_weights,
    get_vault_1_balances,
    get_vault_1_unit_tracker,
    get_vault_2_weights,
    get_vault_2_balances,
    get_vault_2_unit_tracker,
    get_vault_1_amp,
    get_vault_2_amp,
):
    def _compute_expected_liquidity_swap(swap_amount):
        return vault_utils.compute_expected_liquidity_swap(
            swap_amount,
            get_vault_1_weights(),
            get_vault_1_balances(),
            vault_1.totalSupply(),
            get_vault_1_unit_tracker(),
            get_vault_2_weights(),
            get_vault_2_balances(),
            vault_2.totalSupply(),
            get_vault_2_unit_tracker(),
            get_vault_1_amp(),
            get_vault_2_amp(),
        )

    yield _compute_expected_liquidity_swap


# NOTE: this fixture is only expected to be used on tests that include the fixture 'vault_2'
@pytest.fixture(scope="module")
def compute_expected_swap_given_U(vault_2, get_vault_2_amp):
    def _compute_expected_swap_given_U(U, to_token):
        return vault_utils.compute_expected_swap_given_U(
            U, vault_2._weight(to_token), to_token.balanceOf(vault_2), get_vault_2_amp()
        )

    yield _compute_expected_swap_given_U
