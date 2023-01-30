import pytest
from brownie import reverts

#TODO do not parametrize the following accross pools (how?)


@pytest.fixture(scope="module")
def set_molly_fee_administrator(pool, deployer, molly):
    pool.setFeeAdministrator(molly, {"from": deployer})



# Default governance fee (set on pool factory) **********************************************************************************

@pytest.mark.parametrize("fee", [0.25, 0.5])    # Max is 0.5
def test_set_default_governance_fee(
    swap_factory,
    deployer,
    fee
):
    fee = int(fee * 10**18)


    swap_factory.setNewDefaultGovernanceFee(fee, {"from": deployer})


    # Check fee is saved on-chain
    assert swap_factory._defaultGovernanceFee() == fee, "Governance fee not saved on-chain."



def test_set_default_governance_fee_over_max(
    swap_factory,
    deployer
):
    fee = int(0.51 * 10**18)     # Maximum is 0.5


    with reverts(dev_revert_msg="dev: GovernanceFee is maximum 50%."):
        swap_factory.setNewDefaultGovernanceFee(fee, {"from": deployer})



def test_set_default_governance_fee_no_auth(
    swap_factory,
    elwood,
):
    fee = int(0.25 * 10**18)


    with reverts("Ownable: caller is not the owner"):
        swap_factory.setNewDefaultGovernanceFee(fee, {"from": elwood})



def test_set_default_governance_fee_event(
    swap_factory,
    deployer
):
    fee_1 = int(0.25 * 10**18)
    swap_factory.setNewDefaultGovernanceFee(fee_1, {"from": deployer})


    # Change the fee twice, as the 'NewDefaultGovernanceFee' reports both the new and the old fee setting
    # (this is to make sure the 'old' reported fee is also correct and not just left blank)
    fee_2 = int(0.25 * 10**18)
    tx_2 = swap_factory.setNewDefaultGovernanceFee(fee_2, {"from": deployer})


    # Check the event
    event = tx_2.events["NewDefaultGovernanceFee"]
    assert event["oldDefaultGovernanceFee"] == fee_1
    assert event["newDefaultGovernanceFee"] == fee_2




# Fee administrator *************************************************************************************************************

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


    with reverts():     # TODO dev revert msg
        pool.setFeeAdministrator(molly, {"from": molly})




# Pool fee **********************************************************************************************************************

#TODO tx.origin == setupMaster
@pytest.mark.usefixtures("set_molly_fee_administrator")
def test_set_pool_fee(
    pool,
    molly
):
    fee = int(0.15 * 10**18)
    assert pool._poolFee() != fee


    pool.setPoolFee(fee, {"from": molly})


    assert pool._poolFee() == fee



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