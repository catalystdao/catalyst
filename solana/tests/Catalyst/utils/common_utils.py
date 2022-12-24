import asyncio
from dataclasses import dataclass

from anchorpy import Program, Provider

from solana.keypair import Keypair
from solana.publickey import PublicKey

from .account_utils import get_swap_pool_authority
from .simulator_utils import create_and_verify_catalyst_simulator
from .swap_pool_utils import CreateAndSetupSwapPoolResult, create_and_setup_swap_pool, CreateConnectionResult, create_connection
from .token_utils import fund_accounts, create_mint, create_token_account

from catalyst_simulator import CatalystSimulator  # type: ignore

SOLANA_CHAIN_ID       = 99
SOLANA_CHAIN_ID_BYTES = SOLANA_CHAIN_ID.to_bytes(8, 'little')

async def create_mints(provider: Provider, mint_authority: Keypair, count: int) -> list[PublicKey]:
    return list(await asyncio.gather(*[
        create_mint(
            provider,
            mint_authority.public_key
        ) for _ in range(count)
    ]))

@dataclass
class UserWallet:
    user_keypair: Keypair
    token_accounts: list[PublicKey]

async def create_users(
    provider: Provider,
    mints: list[PublicKey],
    generic_payer: Keypair,
    count: int
) -> list[UserWallet]:

    async def generate_user_wallet() -> UserWallet:

        user_keypair = Keypair()

        token_accounts = await asyncio.gather(*[
            create_token_account(
                provider,
                mint,
                user_keypair.public_key,
                generic_payer
            ) for mint in mints
        ])

        return UserWallet(user_keypair, list(token_accounts))
    
    return list(await asyncio.gather(*[generate_user_wallet() for _ in range(count)]))

@dataclass
class SimplePoolContext:
    swap_pool_state: PublicKey
    swap_pool_authority: PublicKey
    swap_pool_assets: list[PublicKey]
    users: list[UserWallet]
    create_and_setup_swap_pool_result: CreateAndSetupSwapPoolResult
    create_connection_result: CreateConnectionResult | None
    catalyst_simulator: CatalystSimulator

async def setup_simple_pool_test_env(
    user_count: int,
    asset_count: int,
    pool_assets_balances: list[int],
    pool_assets_weights: list[int],
    pool_amplification: int | None,
    connect_pool_with_itself: bool,
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    swap_interface_program: Program,
    swap_interface_setup_master_keypair: Keypair,
    polymerase_register_program: Program,
    polymerase_sender_program: Program,
    mint_authority: Keypair,
    generic_payer: Keypair
) -> SimplePoolContext:

    assert user_count > 0
    assert len(pool_assets_balances) == asset_count
    assert len(pool_assets_weights) == asset_count

    provider = swap_pool_program.provider

    swap_pool_state_keypair = Keypair()
    swap_pool_state         = swap_pool_state_keypair.public_key

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state,
    )[0]

    # Create new mints and users each time to ensure tests always have a clean start
    swap_pool_assets: list[PublicKey]  = await create_mints(provider, mint_authority, asset_count)
    users: list[UserWallet] = await create_users(provider, swap_pool_assets, generic_payer, user_count)

    liquidity_provider   = users[0]
    await fund_accounts(
        provider                 = provider,
        mints                    = swap_pool_assets,
        mints_authority          = mint_authority,
        token_accounts           = liquidity_provider.token_accounts,
        balance                  = pool_assets_balances,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = pool_assets_balances,
        token_accounts_authority = liquidity_provider.user_keypair
    )

    # Create the pool and deposit assets
    create_and_setup_swap_pool_result = await create_and_setup_swap_pool(
        swap_pool_program,
        swap_pool_setup_master_keypair,
        pool_amplification,
        pool_assets_weights,
        pool_assets_balances,
        swap_pool_assets,
        liquidity_provider.token_accounts,
        liquidity_provider.user_keypair.public_key,
        swap_interface_program,
        swap_interface_setup_master_keypair,
        polymerase_register_program,
        polymerase_sender_program,
        swap_pool_state_keypair=swap_pool_state_keypair
    )

    initialize_swap_pool_state_result = create_and_setup_swap_pool_result.initialize_swap_pool_state_result
    swap_pool_state                   = initialize_swap_pool_state_result.swap_pool_state
    swap_pool_authority               = initialize_swap_pool_state_result.swap_pool_authority
    dao_authority_keypair             = initialize_swap_pool_state_result.dao_authority_keypair
    swap_interface_state              = create_and_setup_swap_pool_result.initialize_swap_interface_result.swap_interface_state


    # Initialise and fund the catalyst simulator, match the deployed pool config and state with the simulator
    catalyst_simulator = await create_and_verify_catalyst_simulator(
        swap_pool_program,
        swap_pool_state,
        swap_interface_state,
        pool_amplification,
        swap_pool_assets,
        pool_assets_weights,
        pool_assets_balances,
        liquidity_provider.user_keypair.public_key
    )

    create_connection_result = None
    if connect_pool_with_itself:
        # Connect pool with itself (via swap interface)
        target_chain_id   = SOLANA_CHAIN_ID
        target_pool_id    = swap_interface_state
        target_program_id = swap_interface_program.program_id

        create_connection_result = await create_connection(
            target_chain_id        = target_chain_id,
            target_pool_id         = target_pool_id,
            target_program_id      = target_program_id,
            swap_pool_program      = swap_pool_program,
            swap_pool_state        = swap_pool_state,
            authority_keypair      = dao_authority_keypair,
            swap_interface_program = swap_interface_program,
            swap_interface_state   = swap_interface_state,
            rent_payer_keypair     = generic_payer
        )
    
    return SimplePoolContext(
        swap_pool_state,
        swap_pool_authority,
        swap_pool_assets,
        users,
        create_and_setup_swap_pool_result,
        create_connection_result,
        catalyst_simulator
    )