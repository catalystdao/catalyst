from dataclasses import dataclass
from anchorpy import Context, Program
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.system_program import SYS_PROGRAM_ID
from solana.rpc.commitment import Commitment
from solders.signature import Signature

from utils.transaction_utils import confirm_transaction, DEFAULT_TX_COMMITMENT

# Result classes ****************************************************************************************************************

@dataclass
class InitializePolymeraseEndpointResult:
    tx: Signature
    polymerase_endpoint_state: PublicKey
    


# Utils *************************************************************************************************************************

async def initialize_polymerase_endpoint(
    polymerase_endpoint_program: Program,
    payer_keypair: Keypair,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> InitializePolymeraseEndpointResult:

    polymerase_endpoint_state_keypair = Keypair()
    
    tx = await polymerase_endpoint_program.rpc["initialize"](
        ctx=Context(
            accounts={
                "payer": payer_keypair.public_key,
                "emulator_state_account": polymerase_endpoint_state_keypair.public_key,
                "system_program": SYS_PROGRAM_ID
            },
            signers=[
                payer_keypair,
                polymerase_endpoint_state_keypair
            ]
        )
    )
    await confirm_transaction(polymerase_endpoint_program.provider, tx, commitment)

    return InitializePolymeraseEndpointResult(tx, polymerase_endpoint_state_keypair.public_key)