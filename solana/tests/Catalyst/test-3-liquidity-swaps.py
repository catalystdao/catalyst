from solana.keypair import Keypair
from solana.transaction import AccountMeta

from anchorpy import Context, Program
from conftest import FactoryFixture
from utils.swap_pool_utils import create_connection
from utils.common_utils import SOLANA_CHAIN_ID
from utils.transaction_utils import TxEventListener, confirm_transaction
from utils.token_utils import approve, get_account_info
from utils.verify_utils import verify_catalyst_state, verify_in_liquidity_swap_event, verify_out_liquidity_swap_event
from spl.token.constants import TOKEN_PROGRAM_ID

from solana.system_program import SYS_PROGRAM_ID


async def test_liquidity_swap_with_self(
    swap_pool_program: Program,
    swap_interface_program: Program,
    polymerase_emulator_setup_master_keypair: Keypair,
    polymerase_sender_program: Program,
    polymerase_receiver_program: Program,
    simple_pool_context_factory: FactoryFixture
):
    # Define test parameters
    provider = swap_pool_program.provider

    user_count           = 2
    asset_count          = 2
    pool_assets_weights  = [1, 1]
    pool_assets_balances = [100000000, 300000000]
    pool_amplification   = None

    simple_pool_context = await simple_pool_context_factory(
        user_count,
        asset_count,
        pool_assets_balances,
        pool_assets_weights,
        pool_amplification,
        True
    )

    swap_pool_state     = simple_pool_context.swap_pool_state
    swap_pool_authority = simple_pool_context.swap_pool_authority
    users               = simple_pool_context.users
    catalyst_simulator  = simple_pool_context.catalyst_simulator

    liquidity_provider  = users[0]

    # Create the pool and deposit assets
    create_and_setup_swap_pool_result    = simple_pool_context.create_and_setup_swap_pool_result

    initialize_swap_pool_state_result    = create_and_setup_swap_pool_result.initialize_swap_pool_state_result
    swap_interface_state                 = create_and_setup_swap_pool_result.initialize_swap_interface_result.swap_interface_state
    port_registration                    = create_and_setup_swap_pool_result.register_polymerase_port_result.polymerase_port

    swap_pool_token_mint                 = initialize_swap_pool_state_result.swap_pool_token_mint
    liquidity_provider_pool_token_wallet = initialize_swap_pool_state_result.depositor_pool_token_wallet_keypair.public_key

    assert simple_pool_context.create_connection_result is not None
    connection_state = simple_pool_context.create_connection_result.connection_state


    # Perform cross-chain liquidity swap (first part): OutLiquiditySwap => CrossChainLiquiditySwap => CallMultichain
    swap_amount       = 100000
    target_chain_id   = SOLANA_CHAIN_ID
    target_pool_id    = swap_interface_state
    target_program_id = swap_interface_program.program_id

    sim_liquidity_units_x64 = catalyst_simulator.out_liquidity_swap(
        swap_amount,
        liquidity_provider.user_keypair.public_key
    )

    liq_provider_account_info_before_swap = await get_account_info(
        provider,
        swap_pool_token_mint,
        liquidity_provider_pool_token_wallet
    )

    await approve(
        swap_pool_program.provider,
        liquidity_provider_pool_token_wallet,
        liquidity_provider.user_keypair,
        swap_pool_authority,
        swap_amount
    )

    polymerase_ibc_data_account_keypair = Keypair() 

    # OutLiquiditySwapEvent
    async with TxEventListener("OutLiquiditySwapEvent") as ev_listener:
        tx = await swap_pool_program.rpc["out_liquidity_swap"](
            target_chain_id,
            target_pool_id,
            liquidity_provider_pool_token_wallet,
            swap_amount,
            ctx=Context(
                accounts={
                    "swap_pool_state_account": swap_pool_state,
                    "liquidity_provider_pool_token_wallet": liquidity_provider_pool_token_wallet,
                    "swap_pool_token_mint": swap_pool_token_mint,
                    "token_program": TOKEN_PROGRAM_ID,
                    "ibc_interface_program": swap_interface_program.program_id,
                    "swap_pool_authority": swap_pool_authority,
                    "interface_state_account": swap_interface_state,
                    "connection_state_account": connection_state,
                    "polymerase_sender_program": polymerase_sender_program.program_id,
                    "polymerase_ibc_data": polymerase_ibc_data_account_keypair.public_key,
                    "polymerase_ibc_data_account_payer": polymerase_emulator_setup_master_keypair.public_key,
                    "system_program": SYS_PROGRAM_ID
                },
                signers=[polymerase_emulator_setup_master_keypair, polymerase_ibc_data_account_keypair]
            )
        )
        await confirm_transaction(swap_pool_program.provider, tx)

        # Verify the event
        out_liquidity_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_out_liquidity_swap_event(
            out_swap_event           = out_liquidity_swap_event,
            swap_pool                = swap_pool_state,
            target_pool              = target_pool_id,
            target_beneficiary       = liquidity_provider_pool_token_wallet,
            target_chain             = target_chain_id,
            pool_token_mint          = swap_pool_token_mint,
            source_pool_token_wallet = liquidity_provider_pool_token_wallet,
            pool_token_amount        = swap_amount,
            liquidity_units_x64      = sim_liquidity_units_x64,
        )

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Make sure provider has provided the pool tokens
    
    liq_provider_account_info_mid_swap = await get_account_info(
        provider,
        swap_pool_token_mint,
        liquidity_provider_pool_token_wallet
    )

    assert liq_provider_account_info_before_swap.amount - liq_provider_account_info_mid_swap.amount == swap_amount


    # Perform cross-chain swap (second part): execute => receive => in_swap
    # ! TODO currently all the accounts are still manually specified here ==> fix once the dispatcher is completed!

    sim_pool_tokens = catalyst_simulator.in_liquidity_swap(
        sim_liquidity_units_x64,
        liquidity_provider.user_keypair.public_key
    )

    # Fetch payload
    data = await polymerase_sender_program.account["IbcData"].fetch(polymerase_ibc_data_account_keypair.public_key)

    authority = Keypair() #not used for now

    async with TxEventListener("InLiquiditySwapEvent") as ev_listener:
        rpc_result = await polymerase_receiver_program.rpc["on_receive"](
            swap_interface_state,
            "",
            "",
            "",
            "",
            1,
            data.payload,
            0,
            0,
            ctx=Context(
                accounts={
                    "port_registration": port_registration,
                    "dapp_program": target_program_id,
                    "authority": authority.public_key
                },
                remaining_accounts=[
                    AccountMeta(swap_interface_state,          is_signer=False, is_writable=False ), # interface_state_account
                    AccountMeta(swap_pool_state,               is_signer=False, is_writable=True  ), # swap_pool
                    AccountMeta(swap_pool_program.program_id,  is_signer=False, is_writable=False ), # swap_pool_program
                    AccountMeta(TOKEN_PROGRAM_ID,              is_signer=False, is_writable=False ), # token_program
                    AccountMeta(connection_state,              is_signer=False, is_writable=False ), # connection_state_account
                    AccountMeta(swap_pool_token_mint,          is_signer=False, is_writable=True ), # output_asset_mint
                    AccountMeta(liquidity_provider_pool_token_wallet,   is_signer=False, is_writable=True  ), # output_asset_wallet
                    AccountMeta(swap_pool_authority,           is_signer=False, is_writable=False ), # swap_pool_authority
                ],
                signers=[authority]
            )
        )
        await confirm_transaction(provider, rpc_result)

        # Verify the event
        in_liquidity_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_in_liquidity_swap_event(
            out_swap_event           = in_liquidity_swap_event,
            swap_pool                = swap_pool_state,
            pool_token_mint          = swap_pool_token_mint,
            target_pool_token_wallet = liquidity_provider_pool_token_wallet,
            liquidity_units_x64      = sim_liquidity_units_x64,
            pool_token_amount        = sim_pool_tokens,
        )

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)


    # Make sure provider has received the pool tokens
    
    liq_provider_account_info_after_swap = await get_account_info(
        provider,
        swap_pool_token_mint,
        liquidity_provider_pool_token_wallet
    )

    # ! TODO liquidity swap not working, less received in in_liquidity_swap than sent in out_liquidity_swap !

    print(swap_amount, sim_pool_tokens.value)
    print(catalyst_simulator.pool_tokens_supply_i.value)

    assert liq_provider_account_info_after_swap.amount - liq_provider_account_info_mid_swap.amount == sim_pool_tokens#swap_amount

    assert liq_provider_account_info_after_swap.amount <= liq_provider_account_info_before_swap.amount
    # assert liq_provider_account_info_after_swap.amount/liq_provider_account_info_before_swap.amount >= 0.95



async def test_liquidity_swap(
    swap_pool_program: Program,
    swap_interface_program: Program,
    polymerase_emulator_setup_master_keypair: Keypair,
    polymerase_sender_program: Program,
    polymerase_receiver_program: Program,
    generic_payer: Keypair,
    simple_pool_context_factory: FactoryFixture
):
    # Define test parameters
    provider = swap_pool_program.provider

    user_count           = 1
    asset_count          = 2
    pool_assets_weights  = [1, 1]
    pool_assets_balances = [100000000, 300000000]
    pool_amplification   = None


    # Setup Pool 1
    spc_1 = await simple_pool_context_factory(
        user_count,
        asset_count,
        pool_assets_balances,
        pool_assets_weights,
        pool_amplification,
        False
    )

    swap_pool_state_1     = spc_1.swap_pool_state
    swap_pool_authority_1 = spc_1.swap_pool_authority
    users_1               = spc_1.users
    catalyst_simulator_1  = spc_1.catalyst_simulator
    liquidity_provider_1  = users_1[0]

    create_and_setup_swap_pool_result_1  = spc_1.create_and_setup_swap_pool_result

    initialize_swap_pool_state_result_1    = create_and_setup_swap_pool_result_1.initialize_swap_pool_state_result
    swap_interface_state_1                 = create_and_setup_swap_pool_result_1.initialize_swap_interface_result.swap_interface_state
    port_registration_1                    = create_and_setup_swap_pool_result_1.register_polymerase_port_result.polymerase_port

    dao_authority_keypair_1                = initialize_swap_pool_state_result_1.dao_authority_keypair
    swap_pool_token_mint_1                 = initialize_swap_pool_state_result_1.swap_pool_token_mint
    liquidity_provider_pool_token_wallet_1 = initialize_swap_pool_state_result_1.depositor_pool_token_wallet_keypair.public_key


    # Setup Pool 2
    spc_2 = await simple_pool_context_factory(
        user_count,
        asset_count,
        pool_assets_balances,
        pool_assets_weights,
        pool_amplification,
        False
    )

    swap_pool_state_2     = spc_2.swap_pool_state
    swap_pool_authority_2 = spc_2.swap_pool_authority
    users_2               = spc_2.users
    catalyst_simulator_2  = spc_2.catalyst_simulator
    liquidity_provider_2  = users_2[0]

    create_and_setup_swap_pool_result_2  = spc_2.create_and_setup_swap_pool_result

    initialize_swap_pool_state_result_2    = create_and_setup_swap_pool_result_2.initialize_swap_pool_state_result
    swap_interface_state_2                 = create_and_setup_swap_pool_result_2.initialize_swap_interface_result.swap_interface_state
    port_registration_2                    = create_and_setup_swap_pool_result_2.register_polymerase_port_result.polymerase_port

    dao_authority_keypair_2                = initialize_swap_pool_state_result_2.dao_authority_keypair
    swap_pool_token_mint_2                 = initialize_swap_pool_state_result_2.swap_pool_token_mint
    liquidity_provider_pool_token_wallet_2 = initialize_swap_pool_state_result_2.depositor_pool_token_wallet_keypair.public_key


    # Connect Pool 1 and Pool 2
    target_chain_id   = SOLANA_CHAIN_ID
    target_program_id = swap_interface_program.program_id

    create_connection_result_1 = await create_connection(
        target_chain_id        = target_chain_id,
        target_pool_id         = swap_interface_state_2,
        target_program_id      = target_program_id,
        swap_pool_program      = swap_pool_program,
        swap_pool_state        = swap_pool_state_1,
        authority_keypair      = dao_authority_keypair_1,
        swap_interface_program = swap_interface_program,
        swap_interface_state   = swap_interface_state_1,
        rent_payer_keypair     = generic_payer
    )
    connection_state_1 = create_connection_result_1.connection_state

    create_connection_result_2 = await create_connection(
        target_chain_id        = target_chain_id,
        target_pool_id         = swap_interface_state_1,
        target_program_id      = target_program_id,
        swap_pool_program      = swap_pool_program,
        swap_pool_state        = swap_pool_state_2,
        authority_keypair      = dao_authority_keypair_2,
        swap_interface_program = swap_interface_program,
        swap_interface_state   = swap_interface_state_2,
        rent_payer_keypair     = generic_payer
    )
    connection_state_2 = create_connection_result_2.connection_state


    # Perform cross-chain liquidity swap (first part): OutLiquiditySwap => CrossChainLiquiditySwap => CallMultichain
    swap_amount    = 100000
    target_pool_id = swap_interface_state_2

    sim_liquidity_units_x64 = catalyst_simulator_1.out_liquidity_swap(
        swap_amount,
        liquidity_provider_1.user_keypair.public_key
    )

    liq_provider_account_info_before_swap = await get_account_info(
        provider,
        swap_pool_token_mint_1,
        liquidity_provider_pool_token_wallet_1
    )

    await approve(
        swap_pool_program.provider,
        liquidity_provider_pool_token_wallet_1,
        liquidity_provider_1.user_keypair,
        swap_pool_authority_1,
        swap_amount
    )

    polymerase_ibc_data_account_keypair = Keypair() 

    # OutLiquiditySwapEvent
    async with TxEventListener("OutLiquiditySwapEvent") as ev_listener:
        tx = await swap_pool_program.rpc["out_liquidity_swap"](
            target_chain_id,
            target_pool_id,
            liquidity_provider_pool_token_wallet_2,
            swap_amount,
            ctx=Context(
                accounts={
                    "swap_pool_state_account": swap_pool_state_1,
                    "liquidity_provider_pool_token_wallet": liquidity_provider_pool_token_wallet_1,
                    "swap_pool_token_mint": swap_pool_token_mint_1,
                    "token_program": TOKEN_PROGRAM_ID,
                    "ibc_interface_program": swap_interface_program.program_id,
                    "swap_pool_authority": swap_pool_authority_1,
                    "interface_state_account": swap_interface_state_1,
                    "connection_state_account": connection_state_1,
                    "polymerase_sender_program": polymerase_sender_program.program_id,
                    "polymerase_ibc_data": polymerase_ibc_data_account_keypair.public_key,
                    "polymerase_ibc_data_account_payer": polymerase_emulator_setup_master_keypair.public_key,
                    "system_program": SYS_PROGRAM_ID
                },
                signers=[polymerase_emulator_setup_master_keypair, polymerase_ibc_data_account_keypair]
            )
        )
        await confirm_transaction(swap_pool_program.provider, tx)

        # Verify the event
        out_liquidity_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_out_liquidity_swap_event(
            out_swap_event           = out_liquidity_swap_event,
            swap_pool                = swap_pool_state_1,
            target_pool              = target_pool_id,
            target_beneficiary       = liquidity_provider_pool_token_wallet_2,
            target_chain             = target_chain_id,
            pool_token_mint          = swap_pool_token_mint_1,
            source_pool_token_wallet = liquidity_provider_pool_token_wallet_1,
            pool_token_amount        = swap_amount,
            liquidity_units_x64      = sim_liquidity_units_x64,
        )

    await verify_catalyst_state(swap_pool_program, swap_pool_state_1, catalyst_simulator_1)

    # Make sure provider has provided the pool tokens
    
    liq_provider_account_info_mid_swap = await get_account_info(
        provider,
        swap_pool_token_mint_1,
        liquidity_provider_pool_token_wallet_1
    )

    assert liq_provider_account_info_before_swap.amount - liq_provider_account_info_mid_swap.amount == swap_amount


    # Perform cross-chain swap (second part): execute => receive => in_swap
    # ! TODO currently all the accounts are still manually specified here ==> fix once the dispatcher is completed!

    sim_pool_tokens = catalyst_simulator_2.in_liquidity_swap(
        sim_liquidity_units_x64,
        liquidity_provider_2.user_keypair.public_key
    )

    # Fetch payload
    data = await polymerase_sender_program.account["IbcData"].fetch(polymerase_ibc_data_account_keypair.public_key)

    authority = Keypair() #not used for now

    async with TxEventListener("InLiquiditySwapEvent") as ev_listener:
        rpc_result = await polymerase_receiver_program.rpc["on_receive"](
            swap_interface_state_2,
            "",
            "",
            "",
            "",
            1,
            data.payload,
            0,
            0,
            ctx=Context(
                accounts={
                    "port_registration": port_registration_2,
                    "dapp_program": target_program_id,
                    "authority": authority.public_key
                },
                remaining_accounts=[
                    AccountMeta(swap_interface_state_2,                  is_signer=False, is_writable=False ), # interface_state_account
                    AccountMeta(swap_pool_state_2,                       is_signer=False, is_writable=True  ), # swap_pool
                    AccountMeta(swap_pool_program.program_id,            is_signer=False, is_writable=False ), # swap_pool_program
                    AccountMeta(TOKEN_PROGRAM_ID,                        is_signer=False, is_writable=False ), # token_program
                    AccountMeta(connection_state_2,                      is_signer=False, is_writable=False ), # connection_state_account
                    AccountMeta(swap_pool_token_mint_2,                  is_signer=False, is_writable=True ),  # output_asset_mint
                    AccountMeta(liquidity_provider_pool_token_wallet_2,  is_signer=False, is_writable=True  ), # output_asset_wallet
                    AccountMeta(swap_pool_authority_2,                   is_signer=False, is_writable=False ), # swap_pool_authority
                ],
                signers=[authority]
            )
        )
        await confirm_transaction(provider, rpc_result)

        # Verify the event
        in_liquidity_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_in_liquidity_swap_event(
            out_swap_event           = in_liquidity_swap_event,
            swap_pool                = swap_pool_state_2,
            pool_token_mint          = swap_pool_token_mint_2,
            target_pool_token_wallet = liquidity_provider_pool_token_wallet_2,
            liquidity_units_x64      = sim_liquidity_units_x64,
            pool_token_amount        = sim_pool_tokens,
        )

    await verify_catalyst_state(swap_pool_program, swap_pool_state_2, catalyst_simulator_2)


    # Make sure provider has received the pool tokens
    
    liq_provider_account_info_after_swap = await get_account_info(
        provider,
        swap_pool_token_mint_2,
        liquidity_provider_pool_token_wallet_2
    )

    # ! TODO liquidity swap not working, less received in in_liquidity_swap than sent in out_liquidity_swap !

    print(swap_amount, sim_pool_tokens.value)
    print(catalyst_simulator_2.pool_tokens_supply_i.value)

    assert liq_provider_account_info_after_swap.amount > liq_provider_account_info_mid_swap.amount

    assert (liq_provider_account_info_after_swap.amount - liq_provider_account_info_mid_swap.amount)/swap_amount > 0.95 #TODO add better test
