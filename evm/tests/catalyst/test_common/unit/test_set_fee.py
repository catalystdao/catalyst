import pytest
from brownie import reverts

pytestmark = pytest.mark.no_vault_param


@pytest.fixture(scope="module")
def set_molly_fee_administrator(vault, deployer, molly):
    vault.setFeeAdministrator(molly, {"from": deployer})


# Default governance fee (set on vault factory) **********************************************************************************


@pytest.mark.parametrize("fee", [0.25, 0.75])  # Max is 0.75
def test_set_default_governance_fee(swap_factory, deployer, fee):
    fee = int(fee * 10**18)
    assert swap_factory._defaultGovernanceFee() != fee

    swap_factory.setDefaultGovernanceFee(fee, {"from": deployer})

    # Check fee is saved on-chain
    assert (
        swap_factory._defaultGovernanceFee() == fee
    ), "Governance fee not saved on-chain."


def test_set_default_governance_fee_over_max(swap_factory, deployer):
    fee = int(0.76 * 10**18)  # Maximum is 0.75

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: Maximum GovernanceFeeSare exceeded."
        swap_factory.setDefaultGovernanceFee(fee, {"from": deployer})


def test_set_default_governance_fee_no_auth(
    swap_factory,
    elwood,
):
    fee = int(0.25 * 10**18)

    with reverts("Ownable: caller is not the owner"):
        swap_factory.setDefaultGovernanceFee(fee, {"from": elwood})


def test_set_default_governance_fee_event(swap_factory, deployer):
    fee = int(0.25 * 10**18)
    assert swap_factory._defaultGovernanceFee() != fee

    tx = swap_factory.setDefaultGovernanceFee(fee, {"from": deployer})

    # Check the event
    event = tx.events["SetDefaultGovernanceFee"]
    assert event["fee"] == fee


# Fee administrator *************************************************************************************************************


def test_default_fee_administrator(vault, deployer):
    assert (
        vault._feeAdministrator() == deployer
    )  # Default fee administrator is the vault deployer


def test_set_fee_administrator(vault, deployer, molly):
    assert vault._feeAdministrator() != molly

    vault.setFeeAdministrator(
        molly, {"from": deployer}
    )  # Only factory owner is allowed to set fee admin

    assert vault._feeAdministrator() == molly


def test_set_fee_administrator_no_auth(vault, molly):
    assert vault._feeAdministrator() != molly

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: Only factory owner"
        vault.setFeeAdministrator(molly, {"from": molly})


def test_set_fee_administrator_event(vault, deployer, molly):
    assert vault._feeAdministrator() != molly

    tx = vault.setFeeAdministrator(
        molly, {"from": deployer}
    )  # Only factory owner is allowed to set fee admin

    event = tx.events["SetFeeAdministrator"]
    assert event["administrator"] == molly


# Vault fee **********************************************************************************************************************


@pytest.mark.usefixtures("set_molly_fee_administrator")
@pytest.mark.parametrize("fee", [0.15, 1])  # Max is 1
def test_set_vault_fee(vault, molly, fee):
    fee = int(fee * 10**18)
    assert vault._vaultFee() != fee

    vault.setVaultFee(fee, {"from": molly})

    assert vault._vaultFee() == fee


@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_vault_fee_over_max(vault, molly):
    fee = int(1.01 * 10**18)  # Max is 1
    assert vault._vaultFee() != fee

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: VaultFee is maximum 100%."
        vault.setVaultFee(fee, {"from": molly})


def test_set_vault_fee_no_auth(vault, molly):
    fee = int(0.15 * 10**18)

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: Only feeAdministrator can set new fee"
        vault.setVaultFee(fee, {"from": molly})


@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_vault_fee_event(vault, molly):
    fee = int(0.15 * 10**18)
    assert vault._vaultFee() != fee  # Make sure the event holds the new fee

    tx = vault.setVaultFee(fee, {"from": molly})

    event = tx.events["SetVaultFee"]
    assert event["fee"] == fee


# Governance fee ****************************************************************************************************************


@pytest.mark.usefixtures("set_molly_fee_administrator")
@pytest.mark.parametrize("fee", [0.15, 0.75])  # Max is 0.75
def test_set_governance_fee(vault, molly, fee):
    fee = int(fee * 10**18)
    assert vault._governanceFeeShare() != fee

    vault.setGovernanceFee(fee, {"from": molly})

    assert vault._governanceFeeShare() == fee


@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_governance_fee_over_max(vault, molly):
    fee = int(0.76 * 10**18)  # Max is 0.75
    assert vault._governanceFeeShare() != fee

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: Maximum GovernanceFeeSare exceeded."
        vault.setGovernanceFee(fee, {"from": molly})


def test_set_governance_fee_no_auth(vault, molly):
    fee = int(0.15 * 10**18)

    with reverts():  # ! Should be filtered with dev_revert_msg="dev: Only feeAdministrator can set new fee"
        vault.setGovernanceFee(fee, {"from": molly})


@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_governance_fee_event(vault, molly):
    fee = int(0.15 * 10**18)
    assert vault._governanceFeeShare() != fee  # Make sure the event holds the new fee

    tx = vault.setGovernanceFee(fee, {"from": molly})

    event = tx.events["SetGovernanceFee"]
    assert event["fee"] == fee
