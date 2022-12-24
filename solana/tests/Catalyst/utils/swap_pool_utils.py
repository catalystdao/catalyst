
from dataclasses import dataclass
from typing import Any, List
from anchorpy import Context, Program
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.transaction import AccountMeta
from solana.rpc.commitment import Commitment
from solana.system_program import SYS_PROGRAM_ID
from spl.token.constants import TOKEN_PROGRAM_ID
from solana.sysvar import SYSVAR_RENT_PUBKEY
from solders.signature import Signature

from utils.account_utils import get_swap_pool_asset_wallet, get_swap_pool_authority, get_swap_pool_escrow_wallet, get_swap_pool_token_mint
from utils.swap_interface_utils import InitializeSwapInterfaceResult, RegisterSwapInterfacePolymerasePortResult, get_connection_state_account, initialize_swap_interface_state, register_swap_interface_polymerase_port
from utils.token_utils import approve, create_token_account, mint_to
from utils.transaction_utils import TxEventListener, confirm_transaction, DEFAULT_TX_COMMITMENT
from utils.verify_utils import int_to_u256_array, verify_local_swap_event, verify_out_swap_event


POOL_MAX_ASSET_COUNT  = 3

# Result classes ****************************************************************************************************************

@dataclass
class InitializeSwapPoolStateResult:
    tx: Signature
    swap_pool_state: PublicKey
    swap_pool_token_mint: PublicKey
    swap_pool_authority: PublicKey
    swap_pool_asset_wallets: list[PublicKey]
    dao_authority_keypair: Keypair
    depositor_pool_token_wallet_keypair: Keypair


@dataclass
class LinkSwapInterfaceToSwapPoolResult:
    tx: Signature

@dataclass
class FinishSwapPoolSetupResult:
    tx: Signature

@dataclass
class CreateConnectionResult:
    tx: Signature
    connection_state: PublicKey

@dataclass
class CreateAndSetupSwapPoolResult:
    initialize_swap_pool_state_result: InitializeSwapPoolStateResult
    initialize_swap_interface_result: InitializeSwapInterfaceResult
    register_polymerase_port_result: RegisterSwapInterfacePolymerasePortResult
    link_swap_interface_to_swap_pool_result: LinkSwapInterfaceToSwapPoolResult
    finish_swap_pool_setup_result: FinishSwapPoolSetupResult

@dataclass
class AddLiquidityToSwapPoolResult:
    tx: Signature
    depositor_pool_token_wallet: PublicKey


@dataclass
class PerformCrossChainSwapResult:
    tx: Signature
    ibc_data: PublicKey
    swap_escrow: PublicKey
    out_swap_event: Any

@dataclass
class PerformLocalSwapResult:
    tx: Signature
    local_swap_event: Any

# Utils *************************************************************************************************************************

async def initialize_swap_pool_state(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    amplification: int | None,
    assets_weights: List[int],
    assets_balances: List[int],
    assets_mints: List[PublicKey],
    deposit_wallets: List[PublicKey],
    depositor_pool_token_wallet_authority: PublicKey,
    depositor_pool_token_wallet_keypair: Keypair | None = None,
    swap_pool_state_keypair: Keypair | None = None,
    swap_pool_token_mint: PublicKey | None = None,
    swap_pool_authority: PublicKey | None = None,
    dao_authority_keypair: Keypair | None = None,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> InitializeSwapPoolStateResult:

    mints_count = len(assets_mints)

    if mints_count > POOL_MAX_ASSET_COUNT or not (mints_count == len(assets_weights) == len(assets_balances)):
        raise Exception("Cannot initialize swap pool, invalid count of mints/wallets passed.")

    swap_pool_state_keypair = swap_pool_state_keypair or Keypair()
    dao_authority_keypair   = dao_authority_keypair or Keypair()

    swap_pool_token_mint = swap_pool_token_mint or get_swap_pool_token_mint(
        swap_pool_program.program_id,
        swap_pool_state_keypair.public_key
    )[0]

    swap_pool_authority = swap_pool_authority or get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state_keypair.public_key,
    )[0]

    depositor_pool_token_wallet_keypair = depositor_pool_token_wallet_keypair or Keypair()

    # The following is for remaining accounts
    remaining_accounts: List[AccountMeta] = []
    swap_pool_asset_wallets = []
    for i, mint in enumerate(assets_mints):

        swap_pool_asset_wallet = get_swap_pool_asset_wallet(
            swap_pool_program.program_id,
            swap_pool_state_keypair.public_key,
            mint
        )[0]
        swap_pool_asset_wallets.append(swap_pool_asset_wallet)
        
        # Asset mint
        remaining_accounts.append(AccountMeta(
            pubkey      = mint,
            is_signer   = False,
            is_writable = False
        ))

        # Swap pool asset wallet
        remaining_accounts.append(AccountMeta(
            pubkey      = swap_pool_asset_wallet,
            is_signer   = False,
            is_writable = True
        ))

        # Depositor asset wallet
        remaining_accounts.append(AccountMeta(
            pubkey      = deposit_wallets[i],
            is_signer   = False,
            is_writable = True
        ))

    args: list[Any] = [1] if amplification is None else [amplification]
    args.append([(assets_balances[i] if i < mints_count else 0) for i in range(POOL_MAX_ASSET_COUNT)])
    args.append([(assets_weights[i]  if i < mints_count else 0) for i in range(POOL_MAX_ASSET_COUNT)])

    tx = await swap_pool_program.rpc["initialize"](
        *args,
        ctx=Context(
            accounts={
                "setup_master": swap_pool_setup_master_keypair.public_key,
                "dao_authority": dao_authority_keypair.public_key,
                "swap_pool_state_account": swap_pool_state_keypair.public_key,
                "swap_pool_token_mint": swap_pool_token_mint,
                "depositor_pool_token_wallet_authority": depositor_pool_token_wallet_authority,
                "depositor_pool_token_wallet": depositor_pool_token_wallet_keypair.public_key,
                "swap_pool_authority": swap_pool_authority,
                "rent": SYSVAR_RENT_PUBKEY,
                "token_program": TOKEN_PROGRAM_ID,
                "system_program": SYS_PROGRAM_ID
            },
            remaining_accounts=remaining_accounts,
            signers=[
                swap_pool_setup_master_keypair,
                dao_authority_keypair,
                swap_pool_state_keypair,
                depositor_pool_token_wallet_keypair
            ]
        )
    )
    await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

    return InitializeSwapPoolStateResult(
        tx,
        swap_pool_state_keypair.public_key,
        swap_pool_token_mint,
        swap_pool_authority,
        swap_pool_asset_wallets,
        dao_authority_keypair,
        depositor_pool_token_wallet_keypair
    )


async def link_swap_interface_to_swap_pool(
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    setup_master_keypair: Keypair,
    swap_interface_state: PublicKey,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> LinkSwapInterfaceToSwapPoolResult:

    tx = await swap_pool_program.rpc["link_ibc_interface"](
        swap_interface_state,
        ctx=Context(
            accounts={
                "swap_pool_state_account": swap_pool_state,
                "setup_master": setup_master_keypair.public_key
            },
            signers=[setup_master_keypair]
        )
    )
    await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

    return LinkSwapInterfaceToSwapPoolResult(tx)


async def finish_swap_pool_setup(
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    setup_master_keypair: Keypair,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> FinishSwapPoolSetupResult:
    tx = await swap_pool_program.rpc["finish_setup"](
        ctx=Context(
            accounts={
                "setup_master": setup_master_keypair.public_key,
                "swap_pool_state_account": swap_pool_state
            },
            signers=[
                setup_master_keypair
            ]
        )
    )
    await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

    return FinishSwapPoolSetupResult(tx)


async def create_connection(
    target_chain_id: int,
    target_pool_id: PublicKey,
    target_program_id: PublicKey,
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    authority_keypair: Keypair, # Either setup_master or dao_authority
    swap_interface_program: Program,
    swap_interface_state: PublicKey,
    rent_payer_keypair: Keypair,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> CreateConnectionResult:

    connection_state, _ = get_connection_state_account(
        swap_interface_program.program_id,
        swap_interface_state,
        target_chain_id,
        target_pool_id
    )

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state
    )[0]

    tx = await swap_pool_program.rpc["create_connection"](
        target_chain_id,
        target_pool_id,
        target_program_id,
        ctx=Context(
            accounts={
                "authority"                : authority_keypair.public_key,
                "swap_pool_state_account"  : swap_pool_state,
                "ibc_interface_program"    : swap_interface_program.program_id,
                "swap_pool_authority"      : swap_pool_authority,
                "rent_payer"               : rent_payer_keypair.public_key,
                "interface_state_account"  : swap_interface_state,
                "connection_state_account" : connection_state,
                "system_program"           : SYS_PROGRAM_ID
            },
            signers=[authority_keypair, rent_payer_keypair]
        )
    )
    await confirm_transaction(swap_interface_program.provider, tx, commitment=commitment)

    return CreateConnectionResult(tx, connection_state)


async def create_and_setup_swap_pool(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    amplification: int | None,
    assets_weights: List[int],
    assets_balances: List[int],
    assets_mints: List[PublicKey],
    deposit_wallets: List[PublicKey],
    depositor_pool_token_wallet_authority: PublicKey,
    swap_interface_program: Program,
    swap_interface_setup_master_keypair: Keypair,
    polymerase_register_program: Program,
    polymerase_sender_program: Program,
    swap_pool_state_keypair: Keypair | None = None,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> CreateAndSetupSwapPoolResult:

    if len(assets_mints) == 0: raise Exception("At least one asset mint must be provided to setup the swap pool.")
    if len(assets_mints) > POOL_MAX_ASSET_COUNT: raise Exception("Too many asset mints provided for the pool setup.")

    initialize_swap_pool_result = await initialize_swap_pool_state(
        swap_pool_program,
        swap_pool_setup_master_keypair,
        amplification,
        assets_weights,
        assets_balances,
        assets_mints,
        deposit_wallets,
        depositor_pool_token_wallet_authority,
        swap_pool_state_keypair=swap_pool_state_keypair
    )
    swap_pool_state = initialize_swap_pool_result.swap_pool_state

    initialize_swap_interface_result = await initialize_swap_interface_state(
        swap_interface_program,
        swap_pool_program,
        swap_pool_state,
        polymerase_sender_program,
        swap_interface_setup_master_keypair,
        commitment=commitment
    )

    register_polymerase_port_result = await register_swap_interface_polymerase_port(
        swap_interface_program,
        initialize_swap_interface_result.swap_interface_state,
        swap_pool_program,
        swap_pool_state,
        polymerase_register_program.program_id,
        swap_interface_setup_master_keypair
    )

    link_swap_interface_to_swap_pool_result = await link_swap_interface_to_swap_pool(
        swap_pool_program,
        swap_pool_state,
        swap_pool_setup_master_keypair,
        initialize_swap_interface_result.swap_interface_state,
        commitment=commitment
    )

    finish_swap_pool_setup_result = await finish_swap_pool_setup(
        swap_pool_program,
        swap_pool_state,
        swap_pool_setup_master_keypair,
        commitment=commitment
    )

    return CreateAndSetupSwapPoolResult(
        initialize_swap_pool_result,
        initialize_swap_interface_result,
        register_polymerase_port_result,
        link_swap_interface_to_swap_pool_result,
        finish_swap_pool_setup_result
    )    


async def add_liquidity_to_swap_pool(
    swap_pool_program: Program,
    amount: int,
    asset_mint: PublicKey,
    source_wallet: PublicKey,
    source_wallet_authority_keypair: Keypair,
    swap_pool_asset_wallet: PublicKey,
    swap_pool_token_mint: PublicKey,
    swap_pool_authority: PublicKey,
    swap_pool_state: PublicKey,
    depositor_pool_token_wallet: PublicKey | None = None,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> AddLiquidityToSwapPoolResult:

    provider = swap_pool_program.provider

    # Create pool token account for the depositor
    if depositor_pool_token_wallet is None:
        depositor_pool_token_wallet = await create_token_account(
            provider,
            swap_pool_token_mint,
            source_wallet_authority_keypair.public_key,
            commitment=commitment
        )

    # Delegate tokens to the SwapPool authority before the deposit instruction call
    await approve(
        provider,
        source_wallet,
        source_wallet_authority_keypair,
        swap_pool_authority,
        amount,
        commitment=commitment
    )

    # Deposit assets
    tx = await swap_pool_program.rpc["deposit"](
        amount,
        ctx=Context(
            accounts={
                "swap_pool_state_account": swap_pool_state,
                "deposited_asset_mint": asset_mint,
                "depositor_asset_wallet": source_wallet,
                "depositor_pool_token_wallet": depositor_pool_token_wallet,
                "swap_pool_asset_wallet": swap_pool_asset_wallet,
                "swap_pool_token_mint": swap_pool_token_mint,
                "swap_pool_authority": swap_pool_authority,
                "token_program": TOKEN_PROGRAM_ID
            },
            signers=[]
        )
    )
    await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

    return AddLiquidityToSwapPoolResult(tx, depositor_pool_token_wallet)



async def perform_local_swap(
    input_amount: int,
    minimum_output_amount: int,
    approx: bool,
    input_asset: PublicKey,
    source_wallet: PublicKey,
    source_wallet_authority_keypair: Keypair,
    output_asset: PublicKey,
    destination_wallet: PublicKey,
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    swap_pool_input_asset_wallet: PublicKey,
    swap_pool_output_asset_wallet: PublicKey,
    swap_pool_authority: PublicKey,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> PerformLocalSwapResult:

    await approve(
        swap_pool_program.provider,
        source_wallet,
        source_wallet_authority_keypair,
        swap_pool_authority,
        input_amount,
        commitment=commitment
    )

    async with TxEventListener("LocalSwapEvent") as ev_listener:
        tx = await swap_pool_program.rpc["local_swap"](
            input_amount,
            minimum_output_amount,
            approx,
            ctx=Context(
                accounts={
                    "swap_pool_state_account": swap_pool_state,
                    "input_asset_mint": input_asset,
                    "input_asset_wallet": source_wallet,
                    "swap_pool_input_asset_wallet": swap_pool_input_asset_wallet,
                    "output_asset_mint": output_asset,
                    "output_asset_wallet": destination_wallet,
                    "swap_pool_output_asset_wallet": swap_pool_output_asset_wallet,
                    "swap_pool_authority": swap_pool_authority,
                    "token_program": TOKEN_PROGRAM_ID
                }
            )
        )
        await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

        # Verify the deposit event
        local_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_local_swap_event(
            local_swap_event        = local_swap_event,
            swap_pool               = swap_pool_state,
            deposited_asset_mint    = input_asset,
            depositor_asset_wallet  = source_wallet,
            withdrawn_asset_mint    = output_asset,
            withdrawer_asset_wallet = destination_wallet,
            deposited_asset_amount  = input_amount
        )

    return PerformLocalSwapResult(tx, local_swap_event)



async def perform_cross_chain_swap(
    input_amount: int,
    input_asset: PublicKey,
    source_wallet: PublicKey,
    source_wallet_authority_keypair: Keypair,
    target_chain_id: int,
    target_pool_id: PublicKey,
    output_asset_index: int,
    destination_wallet: PublicKey,
    swap_escrow_nonce: int,
    swap_escrow_payer: Keypair,
    swap_pool_program: Program,
    swap_pool_state: PublicKey,
    swap_pool_input_asset_wallet: PublicKey,
    swap_pool_authority: PublicKey,
    swap_interface_program: Program,
    swap_interface_state: PublicKey,
    swap_interface_connection_state: PublicKey,
    polymerase_sender_program: Program,
    polymerase_payer: Keypair,
    min_output: int = 0,
    fallback_wallet: PublicKey | None = None,
    approx_from: bool = False,
    approx_to:bool = False,
    commitment: Commitment = DEFAULT_TX_COMMITMENT
) -> PerformCrossChainSwapResult:

    # TODO polymerase accounts should be derived here

    swap_escrow_wallet = get_swap_pool_escrow_wallet(
        swap_pool_program.program_id,
        swap_pool_state,
        swap_escrow_nonce
    )[0]

    fallback_wallet = fallback_wallet or source_wallet

    await approve(
        swap_pool_program.provider,
        source_wallet,
        source_wallet_authority_keypair,
        swap_pool_authority,
        input_amount,
        commitment=commitment
    )

    polymerase_ibc_data_account_keypair = Keypair() 

    async with TxEventListener("OutSwapEvent") as ev_listener:
        tx = await swap_pool_program.rpc["out_swap"](
            target_chain_id,
            target_pool_id,
            output_asset_index,
            destination_wallet,
            input_amount,
            int_to_u256_array(min_output),
            swap_escrow_nonce,
            fallback_wallet,
            approx_from,
            approx_to,
            ctx=Context(
                accounts={
                    "swap_pool_state_account": swap_pool_state,
                    "input_asset_mint": input_asset,
                    "input_asset_wallet": source_wallet,
                    "swap_pool_input_asset_wallet": swap_pool_input_asset_wallet,
                    "token_program": TOKEN_PROGRAM_ID,
                    "swap_escrow": swap_escrow_wallet,
                    "swap_escrow_rent_payer": swap_escrow_payer.public_key,
                    "ibc_interface_program": swap_interface_program.program_id,
                    "swap_pool_authority": swap_pool_authority,
                    "interface_state_account": swap_interface_state,
                    "connection_state_account": swap_interface_connection_state,
                    "polymerase_sender_program": polymerase_sender_program.program_id,
                    "polymerase_ibc_data": polymerase_ibc_data_account_keypair.public_key,
                    "polymerase_ibc_data_account_payer": polymerase_payer.public_key,
                    "system_program": SYS_PROGRAM_ID
                },
                signers=[
                    swap_escrow_payer,
                    polymerase_payer,
                    polymerase_ibc_data_account_keypair
                ]
            )
        )
        await confirm_transaction(swap_pool_program.provider, tx, commitment=commitment)

        # Verify the deposit event
        out_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_out_swap_event(
            out_swap_event          = out_swap_event,
            swap_pool               = swap_pool_state,
            target_pool             = target_pool_id,
            target_asset_index      = output_asset_index,
            target_withdrawer       = destination_wallet,
            target_chain            = target_chain_id,
            deposited_asset_mint    = input_asset,
            depositor_asset_wallet  = source_wallet,
            deposited_asset_amount  = input_amount,
            source_swap_id          = swap_escrow_nonce,
        )

    return PerformCrossChainSwapResult(
        tx,
        polymerase_ibc_data_account_keypair.public_key,
        swap_escrow_wallet,
        out_swap_event
    )

