import brownie
import pytest

from utils.common import MAX_POOL_ASSETS
from utils.deploy_utils import (
    run_deploy_swappool_unsafe,
    run_amp_deploy_swappool,
    run_amp_finish_setup,
)


@pytest.fixture(autouse=True)
def isolation(module_isolation):
    pass


@pytest.fixture(scope="module")
def deployer(accounts):
    yield accounts[1]


@pytest.fixture(scope="module")
def hacker(accounts):
    yield accounts[2]


amplification = 2**62


def test_deploy(swapfactory, crosschaininterface, tokens, deployer, hacker, gov):

    deploy_balances = [10e18, 1000e18, 1000e6, 1e18]

    # Standard deploy
    swap_pool_info = run_amp_deploy_swappool(
        tokens=tokens[:3],
        balances=deploy_balances[:3],
        weights=None,
        amplification=amplification,
        name="",
        symbol="",
        swap_pool_factory=swapfactory,
        cross_chain_interface=crosschaininterface,
        deployer=deployer,
        gov=gov,
    )

    # Call setup twice
    with brownie.reverts(dev_revert_msg="dev: Pool Already setup."):
        swap_pool_info.swappool.setup(
            tokens[:3],
            [1 for _ in tokens[:3]],
            amplification,
            0,
            "",
            "",
            crosschaininterface,
            deployer,
            {"from": deployer},
        )

    # Deploy with invalid amplification
    with brownie.reverts():  # TODO add dev revert message
        run_deploy_swappool_unsafe(
            tokens=tokens[:3],
            balances=deploy_balances[:3],
            weights=None,
            amplification=2**64,
            name="",
            symbol="",
            swap_pool_factory=swapfactory,
            cross_chain_interface=crosschaininterface,
            deployer=deployer,
            gov=gov,
            template_index=1,
        )

    # ! TODO Do we want this?
    # # Deploy with no tokens
    # with brownie.reverts():     #TODO add dev revert message
    #     run_amp_deploy_swappool(
    #         tokens                = [],
    #         balances              = [],
    #         weights               = None,
    #         amplification         = amplification,
    #         name                  = "",
    #         symbol                = "",
    #         swap_pool_factory     = swapfactory,
    #         cross_chain_interface = crosschaininterface,
    #         deployer              = deployer,
    #         gov                   = gov
    #     )

    # Deploy with more assets than supported
    with brownie.reverts():  # TODO add dev revert message
        run_amp_deploy_swappool(
            tokens=tokens[: MAX_POOL_ASSETS + 1],
            balances=deploy_balances[: MAX_POOL_ASSETS + 1],
            weights=None,
            amplification=amplification,
            name="",
            symbol="",
            swap_pool_factory=swapfactory,
            cross_chain_interface=crosschaininterface,
            deployer=deployer,
            gov=gov,
        )

    # Deploy with a token balance set to 0
    with brownie.reverts(dev_revert_msg="dev: 0 tokens provided in setup."):
        run_amp_deploy_swappool(
            tokens=tokens[:3],
            balances=[*deploy_balances[:2], 0],
            weights=None,
            amplification=amplification,
            name="",
            symbol="",
            swap_pool_factory=swapfactory,
            cross_chain_interface=crosschaininterface,
            deployer=deployer,
            gov=gov,
        )


def test_finish_setup(
    swapfactory, crosschaininterface, chainId, tokens, deployer, hacker, gov
):

    deploy_balances = [10e18, 1000e18, 1000e6, 1e18]

    # Deploy (local)
    swap_pool_info = run_amp_deploy_swappool(
        tokens=tokens[:3],
        balances=deploy_balances[:3],
        weights=None,
        amplification=amplification,
        name="",
        symbol="",
        swap_pool_factory=swapfactory,
        cross_chain_interface=crosschaininterface,
        deployer=deployer,
        gov=gov,
    )

    # Invalid caller
    with brownie.reverts(dev_revert_msg="dev: No auth"):
        run_amp_finish_setup(swap_pool_info, hacker)

    # Valid caller
    run_amp_finish_setup(swap_pool_info, deployer)

    # Finish twice
    with brownie.reverts(dev_revert_msg="dev: No auth"):
        run_amp_finish_setup(swap_pool_info, deployer)

    # Deploy (cross-chain)
    swap_pool_info_2 = run_amp_deploy_swappool(
        tokens=tokens[:3],
        balances=deploy_balances[:3],
        weights=None,
        amplification=amplification,
        name="",
        symbol="",
        swap_pool_factory=swapfactory,
        cross_chain_interface=crosschaininterface,
        deployer=deployer,
        gov=gov,
    )

    # Create connection with itself (set onlyLocal to false)
    swap_pool_info_2.swappool.createConnection(
        chainId,
        brownie.convert.to_bytes(swap_pool_info_2.swappool.address.replace("0x", "")),
        True,
        {"from": deployer},
    )

    # Invalid caller
    with brownie.reverts(dev_revert_msg="dev: No auth"):
        run_amp_finish_setup(swap_pool_info_2, hacker)

    # Valid caller
    run_amp_finish_setup(swap_pool_info_2, deployer)


def test_fee_config(swapfactory, crosschaininterface, tokens, deployer, hacker, gov):

    deploy_balances = [10e18, 1000e18, 1000e6, 1e18]

    swap_pool_info = run_amp_deploy_swappool(
        tokens=tokens[:3],
        balances=deploy_balances[:3],
        weights=None,
        amplification=amplification,
        name="",
        symbol="",
        swap_pool_factory=swapfactory,
        cross_chain_interface=crosschaininterface,
        deployer=deployer,
        gov=gov,
        finish_setup=True,
    )
    sp = swap_pool_info.swappool

    # Set fee administrator
    # Invalid caller
    with brownie.reverts():  # TODO dev msg
        sp.setFeeAdministrator(hacker, {"from": hacker})

    # Valid caller
    sp.setFeeAdministrator(gov, {"from": gov})

    assert sp._feeAdministrator() == gov

    # Set pool fee
    pool_fee_x64 = int(0.4 * 2**64)

    # Invalid caller
    with brownie.reverts(dev_revert_msg="dev: Only feeAdministrator can set new fee"):
        sp.setPoolFee(pool_fee_x64, {"from": hacker})

    # Valid caller
    sp.setPoolFee(pool_fee_x64, {"from": gov})
    assert sp._poolFee() == pool_fee_x64

    # TODO check max fee? (to be implemented)

    # Set governance fee # TODO to review how governance fee works
    gov_fee_x64 = int(0.4 * 2**64)

    # Invalid caller
    with brownie.reverts(dev_revert_msg="dev: Only feeAdministrator can set new fee"):
        sp.setGovernanceFee(gov_fee_x64, {"from": hacker})

    # Valid caller
    sp.setGovernanceFee(gov_fee_x64, {"from": gov})
    assert sp._governanceFeeShare() == gov_fee_x64

    # Fee too high
    with brownie.reverts():  # TODO dev msg
        sp.setGovernanceFee(2**64, {"from": gov})
