
from typing import Tuple
from anchorpy import Context, Program
from dataclasses import dataclass
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.system_program import SYS_PROGRAM_ID
from solana.rpc.commitment import Commitment
from solders.signature import Signature

from utils.transaction_utils import confirm_transaction, DEFAULT_TX_COMMITMENT
from utils.account_utils import get_swap_pool_authority


# Result classes ****************************************************************************************************************

@dataclass
class InitializeSwapInterfaceResult:
    tx: Signature
    swap_pool_authority: PublicKey
    swap_interface_state: PublicKey

@dataclass
class RegisterSwapInterfacePolymerasePortResult:
    tx: Signature
    polymerase_port: PublicKey



# Utils *************************************************************************************************************************

async def initialize_swap_interface_state(
    swap_interface_program: Program,
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    polymerase_emulator_program: Program,
    swap_interface_setup_master_keypair: Keypair,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> InitializeSwapInterfaceResult:

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state
    )[0]

    swap_interface_state = PublicKey.find_program_address(
        [
            swap_pool_state.__bytes__()
        ],
        swap_interface_program.program_id
    )[0]

    tx = await swap_interface_program.rpc["initialize"](
        swap_pool_state,
        swap_pool_authority,
        polymerase_emulator_program.program_id,
        ctx=Context(
            accounts={
                "configurator": swap_interface_setup_master_keypair.public_key,
                "interface_state_account": swap_interface_state,
                "system_program": SYS_PROGRAM_ID
            },
            signers=[swap_interface_setup_master_keypair]
        )
    )
    await confirm_transaction(swap_interface_program.provider, tx, commitment=commitment)

    return InitializeSwapInterfaceResult(tx, swap_pool_authority, swap_interface_state)


async def register_swap_interface_polymerase_port(
    swap_interface_program: Program,
    swap_interface_state: PublicKey,
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    polymerase_register_program_id: PublicKey,
    swap_interface_setup_master_keypair: Keypair,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> RegisterSwapInterfacePolymerasePortResult:

    port_registration_account = PublicKey.find_program_address(
        [
            swap_interface_state.__bytes__()
        ],
        polymerase_register_program_id
    )[0]

    tx = await swap_interface_program.rpc["register_receiver"](
        ctx=Context(
            accounts={
                "configurator": swap_interface_setup_master_keypair.public_key,
                "interface_state_account": swap_interface_state,
                "swap_pool": swap_pool_state,
                "register_program": polymerase_register_program_id,
                "port_registration": port_registration_account,
                "payer": swap_interface_setup_master_keypair.public_key,
                "system_program": SYS_PROGRAM_ID
            },
            signers=[swap_interface_setup_master_keypair]
        )
    )
    await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

    return RegisterSwapInterfacePolymerasePortResult(tx, port_registration_account)


def get_connection_state_account(
    swap_interface_program_id: PublicKey,
    swap_interface_state: PublicKey,
    target_chain_id: int,
    target_pool_id: PublicKey
) -> Tuple[PublicKey, int]:

    return PublicKey.find_program_address(
        [
            swap_interface_state.__bytes__(),
            target_chain_id.to_bytes(8, 'little'),
            target_pool_id.__bytes__()
        ],
        swap_interface_program_id
    )
