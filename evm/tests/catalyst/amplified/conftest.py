import pytest
from brownie import (
    ZERO_ADDRESS, convert
)
default_depositValues = [1000 * 10**18, 1000 * 10**18]


@pytest.fixture(scope="module")
def default_amp_swappool(deploy_swappool, token1, token2, token3, accounts):
    def deploy_and_validate_swappool(
        depositValues=default_depositValues,
        deployer=accounts[1],
        weights=None,
    ):
        tokens = []
        if len(depositValues) == 3:
            tokens = [token1, token2, token3]
        if len(depositValues) == 2:
            tokens = [token1, token2]
        if len(depositValues) == 1:
            tokens = [token1]

        sp = deploy_swappool(
            tokens,
            depositValues,
            2**62,
            "POOLNAME",
            "PS",
            weights=weights,
            deployer=deployer,
        )
        sp.finishSetup({"from": deployer})

        # Validate swappool is correctly created
        assert sp.ready()
        assert sp.balanceOf(deployer) == 2**64
        assert sp.balanceOf(accounts[0]) == 0
        for i in range(len(tokens)):
            token = tokens[i]
            depositValue = depositValues[i]
            assert token.balanceOf(sp) == depositValue
            assert token.balanceOf(deployer) == 0

        return sp

    yield deploy_and_validate_swappool()


@pytest.fixture(scope="module")
def default_amp_swappool_self(deploy_swappool, token1, token2, token3, accounts, chainId):
    def deploy_and_validate_swappool(
        depositValues=default_depositValues,
        deployer=accounts[1],
        weights=None,
    ):
        tokens = []
        if len(depositValues) == 3:
            tokens = [token1, token2, token3]
        if len(depositValues) == 2:
            tokens = [token1, token2]
        if len(depositValues) == 1:
            tokens = [token1]

        sp = deploy_swappool(
            tokens,
            depositValues,
            2**62,
            "POOLNAME",
            "PS",
            weights=weights,
            deployer=deployer,
        )

        TARGET_CHAIN_ID = chainId

        sp.createConnectionWithChain(
            TARGET_CHAIN_ID,
            convert.to_bytes(sp.address.replace("0x", "")),
            True,
            {"from": deployer},
        )

        sp.finishSetup({"from": deployer})

        # Validate swappool is correctly created
        assert sp.ready()
        assert sp.balanceOf(deployer) == 2**64
        assert sp.balanceOf(accounts[0]) == 0
        for i in range(len(tokens)):
            token = tokens[i]
            depositValue = depositValues[i]
            assert token.balanceOf(sp) == depositValue
            assert token.balanceOf(deployer) == 0

        return sp

    yield deploy_and_validate_swappool()
