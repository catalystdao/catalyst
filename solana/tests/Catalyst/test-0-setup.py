from hypothesis import given, strategies as st
import pytest
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.commitment import Finalized

from anchorpy import Context, Program, Provider

from conftest import create_mints, create_users
from utils.account_utils import get_swap_pool_authority
from utils.swap_interface_utils import initialize_swap_interface_state, register_swap_interface_polymerase_port
from utils.verify_utils import verify_catalyst_configuration
from utils.transaction_utils import confirm_transaction
from utils.swap_pool_utils import POOL_MAX_ASSET_COUNT, finish_swap_pool_setup, initialize_swap_pool_state, link_swap_interface_to_swap_pool
from utils.token_utils import fund_accounts


@given(asset_count = st.integers(min_value=0, max_value=POOL_MAX_ASSET_COUNT+1))
async def test_create_swap_pool_asset_count(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    provider: Provider,
    mint_authority: Keypair,
    generic_payer: Keypair,
    asset_count: int
):
    """
        Create a new SwapPool with a varying amount of assets
    """

    swap_pool_state_keypair = Keypair()
    swap_pool_state         = swap_pool_state_keypair.public_key

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state,
    )[0]
    
    amplification = None

    # Create new assets and mints on every run to isolate tests from each other
    assets          = await create_mints(provider, mint_authority, asset_count)
    assets_balances = [10000 for _ in range(asset_count)]
    assets_weights  = [1 for _ in range(asset_count)]

    users           = await create_users(provider, assets, generic_payer, 1)

    depositor = users[0]
    depositor_asset_wallets               = depositor.token_accounts[:asset_count]
    depositor_pool_token_wallet_authority = depositor.user_keypair.public_key
    
    await fund_accounts(
        provider                 = provider,
        mints                    = assets,
        mints_authority          = mint_authority,
        token_accounts           = depositor_asset_wallets,
        balance                  = assets_balances,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = assets_balances,
        token_accounts_authority = depositor.user_keypair
    )

    try:

        # Initialize swap pool state
        await initialize_swap_pool_state(
            swap_pool_program                       = swap_pool_program,
            swap_pool_setup_master_keypair          = swap_pool_setup_master_keypair,
            amplification                           = amplification,
            assets_weights                          = assets_weights,
            assets_balances                         = assets_balances,
            assets_mints                            = assets,
            deposit_wallets                         = depositor_asset_wallets,
            depositor_pool_token_wallet_authority   = depositor_pool_token_wallet_authority,
            swap_pool_state_keypair                 = swap_pool_state_keypair
        )

        assert(asset_count <= POOL_MAX_ASSET_COUNT)

    except:
        assert(asset_count == 0 or asset_count > POOL_MAX_ASSET_COUNT)

        # End the test, as the initial step did not successfully execute
        return


    # Verify the added assets
    if asset_count > 0:
        await verify_catalyst_configuration(
            swap_pool_program,
            swap_pool_state,
            amplification,
            assets,
            assets_weights,
            None,
            swap_pool_setup_master_keypair.public_key
        )

    # Link swap interface (required to finish setup)
    swap_interface_state = Keypair().public_key    # Random => The interface is not used in this test
    await link_swap_interface_to_swap_pool(
        swap_pool_program,
        swap_pool_state,
        swap_pool_setup_master_keypair,
        swap_interface_state
    )


    # Finish setup
    rpc_result = await swap_pool_program.rpc["finish_setup"](
        ctx=Context(
            accounts={
                "setup_master": swap_pool_setup_master_keypair.public_key,
                "swap_pool_state_account": swap_pool_state
            },
            signers=[
                swap_pool_setup_master_keypair
            ]
        )
    )
    await confirm_transaction(swap_pool_program.provider, rpc_result)

    await verify_catalyst_configuration(
        swap_pool_program,
        swap_pool_state,
        amplification,
        assets,
        assets_weights,
        swap_interface_state,
        None if asset_count != 0 else swap_pool_setup_master_keypair.public_key
    )



async def test_add_same_asset_twice(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    provider: Provider,
    mint_authority: Keypair,
    generic_payer: Keypair
):
    swap_pool_state_keypair = Keypair()
    swap_pool_state         = swap_pool_state_keypair.public_key

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state,
    )[0]
    
    asset_count = 1

    amplification   = 1

    # Create new assets and mints on every run to isolate tests from each other
    assets          = await create_mints(provider, mint_authority, asset_count)
    assets_balances = [10000 for _ in range(asset_count)]
    assets_weights  = [1 for _ in range(asset_count)]

    users           = await create_users(provider, assets, generic_payer, 1)

    depositor = users[0]
    depositor_asset_wallets               = depositor.token_accounts[:asset_count]
    depositor_pool_token_wallet_authority = depositor.user_keypair.public_key
    
    await fund_accounts(
        provider                 = provider,
        mints                    = assets,
        mints_authority          = mint_authority,
        token_accounts           = depositor_asset_wallets,
        balance                  = assets_balances*2,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = assets_balances*2,
        token_accounts_authority = depositor.user_keypair
    )

    try:
        # Initialize swap pool state
        await initialize_swap_pool_state(
            swap_pool_program                       = swap_pool_program,
            swap_pool_setup_master_keypair          = swap_pool_setup_master_keypair,
            amplification                           = amplification,
            assets_weights                          = [assets_weights[0], assets_weights[0]],
            assets_balances                         = [assets_balances[0], assets_balances[0]],
            assets_mints                            = [assets[0], assets[0]],
            deposit_wallets                         = [depositor_asset_wallets[0], depositor_asset_wallets[0]],
            depositor_pool_token_wallet_authority   = depositor_pool_token_wallet_authority,
            swap_pool_state_keypair                 = swap_pool_state_keypair
        )
        
        raise Exception("Pool initialization did not fail for given duplicate assets.")
    except:
        pass



async def test_pool_setup_configurator(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    provider: Provider,
    mint_authority: Keypair,
    generic_payer: Keypair
):
    """
    Tests:
        - The pool setup cannot be finished without a swap interface
        - Only the setup_master can link the swap interface
        - Only the setup_master can finish the pool setup
        - Assets cannot be added after the pool setup is complete
        - The setup cannot be finished twice
    """
    # ! TODO Add swap_interface linkage tests
    hacker_keypair = Keypair()

    swap_pool_state_keypair = Keypair()
    swap_pool_state         = swap_pool_state_keypair.public_key

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state,
    )[0]
    
    asset_count = 1

    amplification = None

    # Create new assets and mints on every run to isolate tests from each other
    assets          = await create_mints(provider, mint_authority, asset_count)
    assets_balances = [10000 for _ in range(asset_count)]
    assets_weights  = [1 for _ in range(asset_count)]

    users           = await create_users(provider, assets, generic_payer, 1)

    depositor = users[0]
    depositor_asset_wallets               = depositor.token_accounts[:asset_count]
    depositor_pool_token_wallet_authority = depositor.user_keypair.public_key
    
    await fund_accounts(
        provider                 = provider,
        mints                    = assets,
        mints_authority          = mint_authority,
        token_accounts           = depositor_asset_wallets,
        balance                  = assets_balances,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = assets_balances,
        token_accounts_authority = depositor.user_keypair
    )

    # Initialize swap pool state
    initialize_swap_pool_state_result = await initialize_swap_pool_state(
        swap_pool_program                       = swap_pool_program,
        swap_pool_setup_master_keypair          = swap_pool_setup_master_keypair,
        amplification                           = amplification,
        assets_weights                          = assets_weights,
        assets_balances                         = assets_balances,
        assets_mints                            = assets,
        deposit_wallets                         = depositor_asset_wallets,
        depositor_pool_token_wallet_authority   = depositor_pool_token_wallet_authority,
        swap_pool_state_keypair                 = swap_pool_state_keypair
    )
    swap_pool_state = initialize_swap_pool_state_result.swap_pool_state

    # The pool setup cannot be finished without a swap interface
    with pytest.raises(Exception):
        await finish_swap_pool_setup(
            swap_pool_program,
            swap_pool_state,
            swap_pool_setup_master_keypair,
            commitment=Finalized                # TODO required, why?
        )


    # Only the setup_master can link the swap interface
    swap_interface_state = Keypair().public_key    # Random => The interface is not used in this test
    with pytest.raises(Exception):
        await link_swap_interface_to_swap_pool(
            swap_pool_program,
            swap_pool_state,
            hacker_keypair,
            swap_interface_state
        )

    await link_swap_interface_to_swap_pool(
        swap_pool_program,
        swap_pool_state,
        swap_pool_setup_master_keypair,
        swap_interface_state
    )



    # Only setup_master should be able to finish the pool setup
    with pytest.raises(Exception):
        await finish_swap_pool_setup(
            swap_pool_program,
            swap_pool_state,
            hacker_keypair
        )

    await finish_swap_pool_setup(
        swap_pool_program,
        swap_pool_state,
        swap_pool_setup_master_keypair
    )



    # Cannot 'finish' the pool setup twice
    with pytest.raises(Exception):
        await finish_swap_pool_setup(
            swap_pool_program,
            swap_pool_state,
            swap_pool_setup_master_keypair
        )

    
    # Verify end state
    await verify_catalyst_configuration(
        swap_pool_program            = swap_pool_program,
        swap_pool_state              = swap_pool_state,
        expected_amplification       = amplification,
        expected_pool_assets_mints   = assets,
        expected_pool_assets_weights = assets_weights,
        expected_ibc_interface       = swap_interface_state,
        expected_setup_master        = None
    )


async def test_swap_interface_setup(
    swap_pool_program: Program,
    swap_interface_program: Program,
    polymerase_sender_program: Program,
    polymerase_register_program: Program,
    swap_interface_setup_master_keypair: Keypair
):

    swap_pool_state = Keypair().public_key  # Random ==> State not used in this test

    initialize_swap_interface_result = await initialize_swap_interface_state(
        swap_interface_program,
        swap_pool_program,
        swap_pool_state,
        polymerase_sender_program,
        swap_interface_setup_master_keypair
    )
    swap_interface_state = initialize_swap_interface_result.swap_interface_state

    await register_swap_interface_polymerase_port(
        swap_interface_program,
        swap_interface_state,
        swap_pool_program,
        swap_pool_state,
        polymerase_register_program.program_id,
        swap_interface_setup_master_keypair
    )
    



