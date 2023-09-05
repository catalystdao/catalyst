import pytest


# Fees **************************************************************************************************************************

# Fees to be tested, defined in tuples with the form (vault_fee, governance_fee)
FEES = [(0.33, 0.07), (1, 0.75)]  # Max allowed


def vault_fee_ids_fn(param):
    vault_fee = param[0]
    governance_fee = param[1]
    return f"Fee.P{vault_fee:.2f}.G{governance_fee:.2f}"


@pytest.fixture(scope="module", params=FEES, ids=vault_fee_ids_fn)
def vault_set_fees(request, vault, deployer):

    vault_fee = request.param[0]
    governance_fee = request.param[1]

    vault.setVaultFee(int(vault_fee * 10**18), {"from": deployer})
    vault.setGovernanceFee(int(governance_fee * 10**18), {"from": deployer})

    yield {"vault_fee": vault_fee, "governance_fee": governance_fee}


@pytest.fixture(scope="module", params=FEES, ids=vault_fee_ids_fn)
def group_set_fees(request, group_vaults, deployer):

    vault_fee = request.param[0]
    governance_fee = request.param[1]

    for vault in group_vaults:
        vault.setVaultFee(int(vault_fee * 10**18), {"from": deployer})
        vault.setGovernanceFee(int(governance_fee * 10**18), {"from": deployer})

    yield {"vault_fee": vault_fee, "governance_fee": governance_fee}
