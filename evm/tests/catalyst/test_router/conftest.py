import pytest
from brownie import CatalystRouter, WETH9, ZERO_ADDRESS, CatalystSwapPoolVolatile, CatalystSwapPoolAmplified
from brownie import convert

@pytest.fixture(scope="module")
def weth(deployer):
    yield WETH9.deploy({'from': deployer})
    
@pytest.fixture(scope="module")
def catalyst_router(weth, deployer):
    yield CatalystRouter.deploy([weth], {'from': deployer})


@pytest.fixture(scope="module")
def encode_router_payload():
    def _encode_router_payload(commands: list, parameters: list):
        encoded_commands = b""
        encoded_parameters = []
        for command, parameter in zip(commands, parameters):
            encoded_commands += convert.to_bytes(command, type_str="bytes1")
            encoded_parameter = b""
            for param in parameter:
                if type(param) is bytes:
                    encoded_parameter += param
                    continue
                encoded_parameter += convert.to_bytes(param, type_str="bytes32")
            encoded_parameters.append(encoded_parameter)
        
        return [encoded_commands, encoded_parameters]

    yield _encode_router_payload


@pytest.fixture(scope="module")
def deploypool(accounts, swap_factory, volatile_swap_pool_template, amplified_swap_pool_template,  cross_chain_interface, deployer):
    def _deploy_pool(
        tokens,
        token_balances,
        weights,
        amp,
        name,
        symbol,
        deployer = deployer,
        only_local = False,
        template_address = None
    ):
        for i, token in enumerate(tokens):
            token.transfer(deployer, token_balances[i], {"from": accounts[0]})
            token.approve(swap_factory, token_balances[i], {"from": deployer})

        if template_address is None:
            if amp == 10**18:
                template_address = volatile_swap_pool_template.address
            elif amp < 10**18:
                template_address = amplified_swap_pool_template.address
            else:
                raise Exception(f"Unknown swap_pool_type \'{amp}\'.")

        tx = swap_factory.deploy_swappool(
            template_address,
            tokens,
            token_balances,
            weights,
            amp,
            0,  # pool fee
            name,
            symbol,
            ZERO_ADDRESS if only_local else cross_chain_interface,
            {"from": deployer},
        )

        if template_address == volatile_swap_pool_template.address:
            return CatalystSwapPoolVolatile.at(tx.return_value)
        else:
            return CatalystSwapPoolAmplified.at(tx.return_value)

    yield _deploy_pool