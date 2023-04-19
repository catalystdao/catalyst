import pytest
from brownie import CatalystRouter, WETH9, ZERO_ADDRESS, CatalystSwapPoolVolatile, CatalystSwapPoolAmplified, p2
from brownie import convert
from eth_abi import encode

@pytest.fixture(scope="module")
def weth(deployer):
    yield WETH9.deploy({'from': deployer})
    
@pytest.fixture(scope="module")
def permit2(deployer):
    yield p2.deploy({'from': deployer})
    
@pytest.fixture(scope="module")
def catalyst_router(permit2, weth, deployer):
    yield CatalystRouter.deploy([permit2, weth], {'from': deployer})

encode_table = {
    0: ["address", "address", "address", "uint256", "uint256"],  # LOCALSWAP
    1: ["address", "bytes32", "bytes32", "bytes32", "address", "uint256", "uint256", "uint256", "address"],  # SENDASSET
    2: ["address", "address", "uint160"],  # PERMIT2_TRANSFER_FROM
    3: ["IAllowanceTransfer.PermitBatch", "bytes"],  # PERMIT2_PERMIT_BATCH
    4: ["address", "address", "uint256"],  # SWEEP
    5: ["address", "address", "uint256"],  # TRANSFER
    6: ["address", "address", "uint256"],  # PAY_PORTION
    7: ["IAllowanceTransfer.PermitSingle", "bytes"],  # PERMIT2_PERMIT
    8: ["address", "uint256"],  # UNWRAP_GAS
    9: ["address", "uint256"],  # WRAP_GAS
    10: ["address", "uint256", "uint256[]", "uint256[]"],  # WITHDRAW_EQUAL
    11: ["address", "uint256", "uint256[]", "uint256[]"],  # WITHDRAW_MIXED
    12: ["address", "address[]", "uint256[]", "uint256"],  # DEPOSIT_MIXED
    13: ["address", "bytes32"],  # ALLOW_CANCEL
}



@pytest.fixture(scope="module")
def encode_router_payload():
    def _encode_router_payload(commands: list, parameters: list):
        encoded_commands = b""
        encoded_parameters = []
        for command, parameter in zip(commands, parameters):
            encoded_commands += convert.to_bytes(command, type_str="bytes1")
            if command == 1:
                parameter[2] = convert.to_bytes(parameter[2])  # Convert target pool address to bytes32
                parameter[3] = convert.to_bytes(parameter[3])  # Convert target user address to bytes32
            if command == 1 and len(parameter) == 10:  
                # If call is send asset and custom calldata is provided, the calldata
                # needs to be appended the other calldata.
                encoded = encode(encode_table[command], parameter[:-1])
                encoded += parameter[-1]
                encoded_parameters.append(
                    encoded
                )
                continue
            encoded_parameters.append(
                encode(encode_table[command], parameter)
            )
        
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