import asyncio
from typing import AsyncIterator
from pytest import fixture
from anchorpy import Context, Program, Provider, WorkspaceType, close_workspace, create_workspace

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.system_program import SYS_PROGRAM_ID
from solana.rpc.types import TxOpts
from solana.rpc.commitment import Finalized

import sys
import os
sys.path.insert(1, os.path.join(sys.path[0], '../../../simulator'))

# Increase test timeout to 30s, skip hypothesis test shrinkage
from hypothesis import settings, Phase
from datetime import timedelta
settings.register_profile('default', deadline=timedelta(seconds=30), phases=[Phase.explicit, Phase.reuse, Phase.generate, Phase.target])
settings.load_profile('default')



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
    provider = Provider.local(opts=TxOpts(skip_preflight=True))
    yield provider
    await provider.close()


@fixture(scope="module")
async def math_program(workspace: WorkspaceType) -> Program:
    program = workspace["fixed_point_math"]
    program.provider.opts = TxOpts(skip_preflight=True)
    return program


@fixture(scope="module")
async def result_account(math_program: Program) -> PublicKey:
    
    account_keypair = Keypair()
    tx_result = await math_program.rpc["initialize"](
        ctx=Context(
                accounts={
                    "calculation_data": account_keypair.public_key,
                    "deployer": math_program.provider.wallet.public_key,
                    "system_program": SYS_PROGRAM_ID
                },
                signers=[
                    account_keypair
                ]
            )
    )

    await math_program.provider.connection.confirm_transaction(tx_result, commitment=Finalized)

    return account_keypair.public_key