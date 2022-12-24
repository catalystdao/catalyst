import asyncio
from datetime import timedelta
from typing import Any, AsyncIterator, Callable, Coroutine
from anchorpy import Context, Program, Provider, WorkspaceType, close_workspace, create_workspace
from hypothesis import settings
from pytest import fixture

settings.register_profile('default', deadline=timedelta(seconds=30)) # Increase test timeout to 30s
settings.load_profile('default')

from utils.transaction_utils import DEFAULT_SKIP_PREFLIGHT, confirm_transaction

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.system_program import SYS_PROGRAM_ID
from solana.rpc.types import TxOpts

import sys
import os
sys.path.insert(1, os.path.join(sys.path[0], '../../../simulator'))

from utils.common_utils import SimplePoolContext, UserWallet, create_mints, create_users, setup_simple_pool_test_env

# Global parameters *************************************************************************************************************

TEST_MINTS_COUNT      = 4
USER_COUNT            = 4
U64_MAX               = 2**64-1



# Global fixtures ***************************************************************************************************************

@fixture(scope="module")
def event_loop():
    """Create an instance of the default event loop for each test case."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@fixture(scope="module")
async def workspace() -> AsyncIterator[WorkspaceType]:
    workspace = create_workspace()
    yield workspace
    await close_workspace(workspace)


@fixture(scope="module")
async def provider() -> AsyncIterator[Provider]:
    provider = Provider.local(opts=TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT))
    yield provider
    await provider.close()



# Programs **********************************************************************************************************************

@fixture(scope="module")
async def swap_pool_program(workspace: WorkspaceType) -> Program:
    program = workspace["swap_pool"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program

@fixture(scope="module")
async def swap_pool_amplified_program(workspace: WorkspaceType) -> Program:
    program = workspace["swap_pool_amplified"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program


@fixture(scope="module")
async def swap_interface_program(workspace: WorkspaceType) -> Program:
    program = workspace["ibc_interface"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program


@fixture(scope="module")
async def polymerase_emulator_program(workspace: WorkspaceType) -> Program:
    program = workspace["polymerase_emulator"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program


@fixture(scope="module")
async def polymerase_register_program(workspace: WorkspaceType) -> Program:
    program = workspace["polymerase_register"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program


@fixture(scope="module")
async def polymerase_sender_program(workspace: WorkspaceType) -> Program:
    program = workspace["polymerase_sender"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program


@fixture(scope="module")
async def polymerase_receiver_program(workspace: WorkspaceType) -> Program:
    program = workspace["polymerase_receiver"]
    program.provider.opts = TxOpts(skip_preflight=DEFAULT_SKIP_PREFLIGHT)
    return program



# Users *************************************************************************************************************************

@fixture(scope="module")
async def generic_payer(provider: Provider) -> Keypair:
    
    payer = Keypair()
    airdrop_result = await provider.connection.request_airdrop(
        payer.public_key, 1000000000000
    )

    await confirm_transaction(provider, airdrop_result.value)

    return payer


@fixture(scope="module")
async def mint_authority() -> Keypair:
    return Keypair()


@fixture(scope="module")
async def swap_pool_setup_master_keypair(provider: Provider) -> Keypair:
    
    setup_master = Keypair()
    airdrop_result = await provider.connection.request_airdrop(
        setup_master.public_key, 1000000000000
    )

    await confirm_transaction(provider, airdrop_result.value)

    return setup_master


@fixture(scope="module")
async def swap_interface_setup_master_keypair(provider: Provider) -> Keypair:
    
    setup_master = Keypair()
    airdrop_result = await provider.connection.request_airdrop(
        setup_master.public_key, 1000000000000
    )

    await confirm_transaction(provider, airdrop_result.value)

    return setup_master


@fixture(scope="module")
async def polymerase_emulator_setup_master_keypair(provider: Provider) -> Keypair:
    
    setup_master = Keypair()
    airdrop_result = await provider.connection.request_airdrop(
        setup_master.public_key, 1000000000000
    )

    await confirm_transaction(provider, airdrop_result.value)

    return setup_master



@fixture(scope="module")
async def mints(
    provider: Provider,
    mint_authority: Keypair
) -> list[PublicKey]:

    return await create_mints(provider, mint_authority, TEST_MINTS_COUNT)



@fixture(scope="module")
async def users(
    provider: Provider,
    mints: list[PublicKey],
    generic_payer: Keypair
) -> list[UserWallet]:

    return await create_users(provider, mints, generic_payer, USER_COUNT)



# Program initialization fixtures ***********************************************************************************************

@fixture(scope="module")
async def polymerase_endpoint_state(
    polymerase_emulator_program: Program,
    generic_payer: Keypair
) -> PublicKey:
    accountKeypair = Keypair()

    await polymerase_emulator_program.rpc["initialize"](
        ctx=Context(
            accounts={
                "payer": generic_payer.public_key,
                "emulator_state_account": accountKeypair.public_key,
                "system_program": SYS_PROGRAM_ID
            },
            signers=[generic_payer, accountKeypair]
        )
    )

    return accountKeypair.public_key


FactoryFixture = Callable[[int, int, list[int], list[int], int | None, bool], Coroutine[Any, Any, SimplePoolContext]]

@fixture(scope="module")
async def simple_pool_context_factory(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    swap_interface_program: Program,
    swap_interface_setup_master_keypair: Keypair,
    polymerase_register_program: Program,
    polymerase_sender_program: Program,
    mint_authority: Keypair,
    generic_payer: Keypair
) -> FactoryFixture:

    async def factory(
        user_count: int,
        asset_count: int,
        pool_assets_balances: list[int],
        pool_assets_weights: list[int],
        pool_amplification: int | None,
        connect_pool_with_itself: bool
    ) -> SimplePoolContext:

        return await setup_simple_pool_test_env(
            user_count,
            asset_count,
            pool_assets_balances,
            pool_assets_weights,
            pool_amplification,
            connect_pool_with_itself,
            swap_pool_program,
            swap_pool_setup_master_keypair,
            swap_interface_program,
            swap_interface_setup_master_keypair,
            polymerase_register_program,
            polymerase_sender_program,
            mint_authority,
            generic_payer
        )
    
    return factory