import pytest
from brownie import reverts

pytestmark = pytest.mark.no_pool_param


@pytest.fixture(scope="module")
def set_molly_fee_administrator(pool, deployer, molly):
    pool.setFeeAdministrator(molly, {"from": deployer})


# Default governance fee (set on pool factory) **********************************************************************************

@pytest.mark.parametrize("fee", [0.25, 0.75])    # Max is 0.75
def test_set_default_governance_fee(
    swap_factory,
    deployer,
    fee
):
    fee = int(fee * 10**18)
    assert swap_factory._defaultGovernanceFee() != fee


    swap_factory.setDefaultGovernanceFee(fee, {"from": deployer})


    # Check fee is saved on-chain
    assert swap_factory._defaultGovernanceFee() == fee, "Governance fee not saved on-chain."



def test_set_default_governance_fee_over_max(
    swap_factory,
    deployer
):
    fee = int(0.76 * 10**18)     # Maximum is 0.75


    with reverts(dev_revert_msg="dev: GovernanceFee is maximum 75%."):
        swap_factory.setDefaultGovernanceFee(fee, {"from": deployer})



def test_set_default_governance_fee_no_auth(
    swap_factory,
    elwood,
):
    fee = int(0.25 * 10**18)


    with reverts("Ownable: caller is not the owner"):
        swap_factory.setDefaultGovernanceFee(fee, {"from": elwood})



def test_set_default_governance_fee_event(
    swap_factory,
    deployer
):
    fee = int(0.25 * 10**18)
    assert swap_factory._defaultGovernanceFee() != fee


    tx = swap_factory.setDefaultGovernanceFee(fee, {"from": deployer})


    # Check the event
    event = tx.events["SetDefaultGovernanceFee"]
    assert event["fee"] == fee




# Fee administrator *************************************************************************************************************

def test_default_fee_administrator(
    pool,
    deployer
):
    assert pool._feeAdministrator() == deployer     # Default fee administrator is the pool deployer



def test_set_fee_administrator(
    pool,
    deployer,
    molly
):
    assert pool._feeAdministrator() != molly


    pool.setFeeAdministrator(molly, {"from": deployer})     # Only factory owner is allowed to set fee admin


    assert pool._feeAdministrator() == molly



def test_set_fee_administrator_no_auth(
    pool,
    molly
):
    assert pool._feeAdministrator() != molly


    with reverts(dev_revert_msg="dev: Only factory owner"):
        pool.setFeeAdministrator(molly, {"from": molly})



def test_set_fee_administrator_event(
    pool,
    deployer,
    molly
):
    assert pool._feeAdministrator() != molly


    tx = pool.setFeeAdministrator(molly, {"from": deployer})     # Only factory owner is allowed to set fee admin


    event = tx.events["SetFeeAdministrator"]
    assert event["administrator"] == molly




# Pool fee **********************************************************************************************************************

#TODO tx.origin == setupMaster
@pytest.mark.usefixtures("set_molly_fee_administrator")
@pytest.mark.parametrize("fee", [0.15, 1])    # Max is 1
def test_set_pool_fee(
    pool,
    molly,
    fee
):
    fee = int(fee * 10**18)
    assert pool._poolFee() != fee


    pool.setPoolFee(fee, {"from": molly})


    assert pool._poolFee() == fee



@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_pool_fee_over_max(
    pool,
    molly
):
    fee = int(1.01 * 10**18)            # Max is 1
    assert pool._poolFee() != fee


    with reverts(dev_revert_msg="dev: PoolFee is maximum 100%."):
        pool.setPoolFee(fee, {"from": molly})



def test_set_pool_fee_no_auth(
    pool,
    molly
):
    fee = int(0.15 * 10**18)


    with reverts(dev_revert_msg="dev: Only feeAdministrator can set new fee"): 
        pool.setPoolFee(fee, {"from": molly})



@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_pool_fee_event(
    pool,
    molly
):
    fee = int(0.15 * 10**18)
    assert pool._poolFee() != fee     # Make sure the event holds the new fee


    tx = pool.setPoolFee(fee, {"from": molly})


    event = tx.events["SetPoolFee"]
    assert event["fee"] == fee




# Governance fee ****************************************************************************************************************

@pytest.mark.usefixtures("set_molly_fee_administrator")
@pytest.mark.parametrize("fee", [0.15, 0.75])    # Max is 0.75
def test_set_governance_fee(
    pool,
    molly,
    fee
):
    fee = int(fee * 10**18)
    assert pool._governanceFee() != fee


    pool.setGovernanceFee(fee, {"from": molly})


    assert pool._governanceFee() == fee



@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_governance_fee_over_max(
    pool,
    molly
):
    fee = int(0.76 * 10**18)            # Max is 0.75
    assert pool._governanceFee() != fee


    with reverts(dev_revert_msg="dev: GovernanceFee is maximum 75%."):
        pool.setGovernanceFee(fee, {"from": molly})



def test_set_governance_fee_no_auth(
    pool,
    molly
):
    fee = int(0.15 * 10**18)


    with reverts(dev_revert_msg="dev: Only feeAdministrator can set new fee"): 
        pool.setGovernanceFee(fee, {"from": molly})



@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_governance_fee_event(
    pool,
    molly
):
    fee = int(0.15 * 10**18)
    assert pool._governanceFee() != fee     # Make sure the event holds the new fee


    tx = pool.setGovernanceFee(fee, {"from": molly})


    event = tx.events["SetGovernanceFee"]
    assert event["fee"] == fee