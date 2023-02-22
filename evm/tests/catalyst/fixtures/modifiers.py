import pytest


# Fees **************************************************************************************************************************

# Fees to be tested, defined in tuples with the form (pool_fee, governance_fee)
FEES = [
    (0.33, 0.07),
    (1, 0.75)       # Max allowed
]   


def pool_fee_ids_fn(param):
    pool_fee = param[0]
    governance_fee = param[1]
    return f"Fee.P{pool_fee:.2f}.G{governance_fee:.2f}"


@pytest.fixture(scope="module", params=FEES, ids=pool_fee_ids_fn)
def pool_set_fees(request, pool, deployer):

    pool_fee = request.param[0]
    governance_fee = request.param[1]

    pool.setPoolFee(int(pool_fee * 10**18), sender=deployer)
    pool.setGovernanceFee(int(governance_fee * 10**18), sender=deployer)

    yield {
        "pool_fee": pool_fee,
        "governance_fee": governance_fee
    }


@pytest.fixture(scope="module", params=FEES, ids=pool_fee_ids_fn)
def group_set_fees(request, group_pools, deployer):

    pool_fee = request.param[0]
    governance_fee = request.param[1]

    for pool in group_pools:
        pool.setPoolFee(int(pool_fee * 10**18), sender=deployer)
        pool.setGovernanceFee(int(governance_fee * 10**18), sender=deployer)

    yield {
        "pool_fee": pool_fee,
        "governance_fee": governance_fee
    }
