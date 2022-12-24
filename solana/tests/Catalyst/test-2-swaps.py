from solana.keypair import Keypair
from solana.transaction import AccountMeta
from solana.system_program import SYS_PROGRAM_ID

from anchorpy import Context, Program
from conftest import FactoryFixture, generic_payer
from utils.common_utils import SOLANA_CHAIN_ID

from utils.swap_pool_utils import perform_cross_chain_swap, perform_local_swap
from utils.transaction_utils import TxEventListener, confirm_transaction
from utils.token_utils import get_account_info, mint_to
from utils.verify_utils import u256_array_to_int, verify_catalyst_state, verify_in_swap_event
from spl.token.constants import TOKEN_PROGRAM_ID

from hypothesis import given, strategies as st


# ! Add local swap test for the same asset as input/output

# TODO ranges
# @given(
#     asset_1_swap_balance = st.integers(min_value=50000000, max_value=10*10**9),
#     pool_asset_1_balance = st.integers(min_value=1000*10**10, max_value=1000*10**12),
#     pool_asset_2_balance = st.integers(min_value=1000*10**10, max_value=1000*10**12),
# )
async def test_local_swap(
    swap_pool_program: Program,
    mint_authority: Keypair,
    simple_pool_context_factory: FactoryFixture,
    asset_1_swap_balance: int = 5000000000,
    pool_asset_1_balance: int = 1000*10**10,
    pool_asset_2_balance: int = 1000*10**10
):
    # Define test parameters
    provider = swap_pool_program.provider

    user_count           = 2
    asset_count          = 2
    pool_assets_weights  = [1, 1]
    pool_assets_balances = [pool_asset_1_balance, pool_asset_2_balance]
    # pool_assets_balances = [1000, 3000]
    # pool_assets_balances = [1000*10**10, 10*10**8]
    pool_amplification   = None

    spc = await simple_pool_context_factory(
        user_count,
        asset_count,
        pool_assets_balances,
        pool_assets_weights,
        pool_amplification,
        False
    )

    swap_pool_state     = spc.swap_pool_state
    swap_pool_authority = spc.swap_pool_authority
    swap_pool_assets    = spc.swap_pool_assets
    users               = spc.users
    catalyst_simulator  = spc.catalyst_simulator

    liquidity_provider  = users[0]

    # Swap config
    swapper                        = users[1]
    swapper_input_asset_index      = 0
    swapper_input_asset            = swap_pool_assets[swapper_input_asset_index]
    swapper_input_asset_wallet     = swapper.token_accounts[swapper_input_asset_index]
    swapper_input_asset_balance    = asset_1_swap_balance
    swapper_output_asset_index     = 1
    swapper_output_asset           = swap_pool_assets[swapper_output_asset_index]
    swapper_output_asset_wallet    = swapper.token_accounts[swapper_output_asset_index]
    swapper_output_asset_min_yield = 0

    initialize_swap_pool_state_result = spc.create_and_setup_swap_pool_result.initialize_swap_pool_state_result
    swap_pool_input_asset_wallet      = initialize_swap_pool_state_result.swap_pool_asset_wallets[swapper_input_asset_index]
    swap_pool_output_asset_wallet     = initialize_swap_pool_state_result.swap_pool_asset_wallets[swapper_output_asset_index]


    # Fund swapper with tokens for the swap
    await mint_to(
        provider,
        swapper_input_asset_wallet,
        swapper_input_asset,
        mint_authority,
        swapper_input_asset_balance
    )

    # Simulate swap     # ! TODO move to catalyst simulator?
    a = pool_assets_balances[0]
    b = pool_assets_balances[1]
    x = swapper_input_asset_balance
    
    # w = catalyst_simulator.get_asset_equilibrium_balance_i(swapper_input_asset).value / catalyst_simulator.get_asset_equilibrium_balance_i(swapper_output_asset).value
    # y = b * (1 - (a / (a + x)) ** w)

    # B0 = b
    # a = B0
    # b = B0 + x
    
    # TODO move to CatalystSimulator testing
    # allowed_deviation = 0.02 / 100

    # Perform local swap
    swapper_output_account_info_before_swap = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )

    await perform_local_swap(
        swapper_input_asset_balance,
        swapper_output_asset_min_yield,
        False, # approx
        swapper_input_asset,
        swapper_input_asset_wallet,
        swapper.user_keypair,
        swapper_output_asset,
        swapper_output_asset_wallet,
        swap_pool_program,
        swap_pool_state,
        swap_pool_input_asset_wallet,
        swap_pool_output_asset_wallet,
        swap_pool_authority
    )

    catalyst_simulator.local_swap(
        from_asset=swapper_input_asset,
        to_asset=swapper_output_asset,
        amount=swapper_input_asset_balance,
        minimum_yield=swapper_output_asset_min_yield
    )
    
    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Compare the current state of the output asset destination wallet
    swapper_output_account_info_after_swap = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )

    swap_output_yield = swapper_output_account_info_after_swap.amount - swapper_output_account_info_before_swap.amount
    assert swap_output_yield >= swapper_output_asset_min_yield

    # print("output yield", swap_output_yield)

    # TODO move to CatalystSimulator testing
    # if (swapper_input_asset_balance < catalyst_simulator.assets_balances_i[swapper_input_asset] / 1000) or y == 0:
    #     assert swap_output_yield <= y
    # else:
    #     assert 1 + allowed_deviation >= swap_output_yield / y >= 1 - allowed_deviation * 100


    # Reverse swap
    swapper_input_account_info_before_reverse_swap = await get_account_info(
        provider,
        swapper_input_asset,
        swapper_input_asset_wallet
    )

    await perform_local_swap(
        swap_output_yield,
        swapper_output_asset_min_yield,
        False, # approx
        swapper_output_asset,
        swapper_output_asset_wallet,
        swapper.user_keypair,
        swapper_input_asset,
        swapper_input_asset_wallet,
        swap_pool_program,
        swap_pool_state,
        swap_pool_output_asset_wallet,
        swap_pool_input_asset_wallet,
        swap_pool_authority
    )

    catalyst_simulator.local_swap(
        from_asset=swapper_output_asset,
        to_asset=swapper_input_asset,
        amount=swap_output_yield,
        minimum_yield=swapper_output_asset_min_yield
    )
    
    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    swapper_input_account_info_after_reverse_swap = await get_account_info(
        provider,
        swapper_input_asset,
        swapper_input_asset_wallet
    )

    reverse_swap_yield = swapper_input_account_info_after_reverse_swap.amount - swapper_input_account_info_before_reverse_swap.amount

    assert reverse_swap_yield <= swapper_input_asset_balance
    # assert 1 - reverse_swap_yield / swapper_input_asset_balance <= 0.05   # TODO move to CatalystSimulator testing

    # print(reverse_swap_yield)


async def test_self_cross_chain_swap(
    swap_pool_program: Program,
    swap_interface_program: Program,
    polymerase_emulator_setup_master_keypair: Keypair,
    polymerase_sender_program: Program,
    polymerase_receiver_program: Program,
    mint_authority: Keypair,
    generic_payer: Keypair,
    simple_pool_context_factory: FactoryFixture,
):
    # Define test parameters
    provider = swap_pool_program.provider

    user_count           = 2
    asset_count          = 2
    pool_assets_weights  = [1, 1]
    pool_assets_balances = [1000, 3000]
    pool_amplification   = None

    spc = await simple_pool_context_factory(
        user_count,
        asset_count,
        pool_assets_balances,
        pool_assets_weights,
        pool_amplification,
        True
    )

    swap_pool_state     = spc.swap_pool_state
    swap_pool_authority = spc.swap_pool_authority
    swap_pool_assets    = spc.swap_pool_assets
    users               = spc.users
    catalyst_simulator  = spc.catalyst_simulator

    # Swap config
    swapper                     = users[1]
    swapper_input_asset_index   = 0
    swapper_input_asset         = swap_pool_assets[swapper_input_asset_index]
    swapper_input_asset_wallet  = swapper.token_accounts[swapper_input_asset_index]
    swapper_input_asset_balance = 500
    swapper_output_asset_index  = 0
    swapper_output_asset        = swap_pool_assets[swapper_output_asset_index]
    swapper_output_asset_wallet = swapper.token_accounts[swapper_output_asset_index]

    initialize_swap_pool_state_result = spc.create_and_setup_swap_pool_result.initialize_swap_pool_state_result
    swap_pool_input_asset_wallet      = initialize_swap_pool_state_result.swap_pool_asset_wallets[swapper_input_asset_index]
    swap_pool_output_asset_wallet     = initialize_swap_pool_state_result.swap_pool_asset_wallets[swapper_output_asset_index]

    swap_interface_state              = spc.create_and_setup_swap_pool_result.initialize_swap_interface_result.swap_interface_state
    port_registration                 = spc.create_and_setup_swap_pool_result.register_polymerase_port_result.polymerase_port

    assert spc.create_connection_result is not None
    connection_state = spc.create_connection_result.connection_state


    # Fund swapper with tokens for the swap
    await mint_to(
        provider,
        swapper_input_asset_wallet,
        swapper_input_asset,
        mint_authority,
        swapper_input_asset_balance
    )


    # Step 1: Perform cross-chain swap Outswap => CrossChainSwap => CallMultichain
    swapper_output_account_info_before_swap = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )


    target_chain_id   = SOLANA_CHAIN_ID
    target_pool_id    = swap_interface_state
    target_program_id = swap_interface_program.program_id

    source_swap_id    = 0

    cross_chain_swap_result = await perform_cross_chain_swap(
        swapper_input_asset_balance,
        swapper_input_asset,
        swapper_input_asset_wallet,
        swapper.user_keypair,
        target_chain_id,
        target_pool_id,
        swapper_output_asset_index,
        swapper_output_asset_wallet,
        source_swap_id,  # Escrow nonce
        generic_payer,
        swap_pool_program,
        swap_pool_state,
        swap_pool_input_asset_wallet,
        swap_pool_authority,
        swap_interface_program,
        swap_interface_state,
        connection_state,
        polymerase_sender_program,
        polymerase_emulator_setup_master_keypair # Payer, can be anyone
    )

    sim_out_swap_units = catalyst_simulator.out_swap(
        swapper_input_asset,
        swapper_input_asset_balance,
        source_swap_id
    )

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Compare simulated out_swap units with the actual ones (from out_swap event)
    out_swap_units = u256_array_to_int(cross_chain_swap_result.out_swap_event.data.withdrawnPoolUnitsX64)
    assert sim_out_swap_units == out_swap_units



    # Step 2: Perform cross-chain swap execute => receive => in_swap
    # ! TODO currently all the accounts are still manually specified here ==> fix once the dispatcher is completed!

    # Fetch payload
    data = await polymerase_sender_program.account["IbcData"].fetch(cross_chain_swap_result.ibc_data)

    authority = Keypair() #not used for now

    async with TxEventListener("InSwapEvent") as ev_listener:
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
                    AccountMeta(swapper_output_asset,          is_signer=False, is_writable=False ), # output_asset_mint
                    AccountMeta(swapper_output_asset_wallet,   is_signer=False, is_writable=True  ), # output_asset_wallet
                    AccountMeta(swap_pool_output_asset_wallet, is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
                    AccountMeta(swap_pool_authority,           is_signer=False, is_writable=False ), # swap_pool_authority
                ],
                signers=[authority]
            )
        )
        await confirm_transaction(provider, rpc_result)

        # Verify the deposit event
        in_swap_event = (await ev_listener.get_events(swap_pool_program))[0]

        verify_in_swap_event(
            in_swap_event            = in_swap_event,
            swap_pool                = swap_pool_state,
            withdrawn_asset_mint     = swapper_output_asset,
            withdrawer_asset_wallet  = swapper_output_asset_wallet,
            deposited_pool_units_x64 = sim_out_swap_units.value
        )

    out_yield = catalyst_simulator.in_swap(
        swapper_output_asset,
        sim_out_swap_units
    )

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Compare the current state of the output asset destination wallet
    swapper_output_account_info_after_swap = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )

    # ! TODO add more tests

    before_balance = swapper_output_account_info_before_swap.amount
    after_balance  = swapper_output_account_info_after_swap.amount

    assert after_balance <= before_balance
    assert out_yield == after_balance
    # assert (after_balance/before_balance >= 0.95)   # TODO move to CatalystSimulator testing



    # Step 3: Trigger ibc ack on source chain (simulates polymer chain endpoint contract)
    swap_escrow = cross_chain_swap_result.swap_escrow
    ack_result = await swap_interface_program.rpc["on_acknowledgement_packet"](
        data.payload,
        ctx=Context(
            accounts={
                "port_registration": port_registration,
                "interface_state_account": swap_interface_state,
                "swap_pool": swap_pool_state,
                "swap_pool_program": swap_pool_program.program_id,
                "swap_pool_authority": swap_pool_authority,
                "token_program": TOKEN_PROGRAM_ID,
                "system_program": SYS_PROGRAM_ID
            },
            remaining_accounts=[
                AccountMeta(swapper_output_asset,          is_signer=False, is_writable=False ), # output_asset_mint
                AccountMeta(swapper_output_asset_wallet,   is_signer=False, is_writable=True  ), # output_asset_wallet
                AccountMeta(swap_pool_output_asset_wallet, is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
                AccountMeta(swap_escrow,                   is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
                AccountMeta(generic_payer.public_key,      is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
            ]
        )
    )
    await confirm_transaction(provider, ack_result)

    catalyst_simulator.out_swap_ack(source_swap_id)

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Compare the current state of the output asset destination wallet
    swapper_output_account_info_after_swap_ack = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )

    # ! TODO add more tests

    before_balance     = swapper_output_account_info_before_swap.amount
    after_ack_balance  = swapper_output_account_info_after_swap_ack.amount

    assert after_balance == after_ack_balance   # Ack must not have an effect on the user's balance




async def test_self_cross_chain_swap_timeout(
    swap_pool_program: Program,
    swap_interface_program: Program,
    polymerase_emulator_setup_master_keypair: Keypair,
    polymerase_sender_program: Program,
    polymerase_receiver_program: Program,
    mint_authority: Keypair,
    generic_payer: Keypair,
    simple_pool_context_factory: FactoryFixture,
):
    # Define test parameters
    provider = swap_pool_program.provider

    user_count           = 2
    asset_count          = 2
    pool_assets_weights  = [1, 1]
    pool_assets_balances = [1000, 3000]
    pool_amplification   = None

    spc = await simple_pool_context_factory(
        user_count,
        asset_count,
        pool_assets_balances,
        pool_assets_weights,
        pool_amplification,
        True
    )

    swap_pool_state     = spc.swap_pool_state
    swap_pool_authority = spc.swap_pool_authority
    swap_pool_assets    = spc.swap_pool_assets
    users               = spc.users
    catalyst_simulator  = spc.catalyst_simulator

    # Swap config
    swapper                     = users[1]
    swapper_input_asset_index   = 0
    swapper_input_asset         = swap_pool_assets[swapper_input_asset_index]
    swapper_input_asset_wallet  = swapper.token_accounts[swapper_input_asset_index]
    swapper_input_asset_balance = 500
    swapper_output_asset_index  = 0
    swapper_output_asset        = swap_pool_assets[swapper_output_asset_index]
    swapper_output_asset_wallet = swapper.token_accounts[swapper_output_asset_index]

    initialize_swap_pool_state_result = spc.create_and_setup_swap_pool_result.initialize_swap_pool_state_result
    swap_pool_input_asset_wallet      = initialize_swap_pool_state_result.swap_pool_asset_wallets[swapper_input_asset_index]
    swap_pool_output_asset_wallet     = initialize_swap_pool_state_result.swap_pool_asset_wallets[swapper_output_asset_index]

    swap_interface_state              = spc.create_and_setup_swap_pool_result.initialize_swap_interface_result.swap_interface_state
    port_registration                 = spc.create_and_setup_swap_pool_result.register_polymerase_port_result.polymerase_port

    assert spc.create_connection_result is not None
    connection_state = spc.create_connection_result.connection_state


    # Fund swapper with tokens for the swap
    await mint_to(
        provider,
        swapper_input_asset_wallet,
        swapper_input_asset,
        mint_authority,
        swapper_input_asset_balance
    )


    # Step 1: Perform cross-chain swap Outswap => CrossChainSwap => CallMultichain
    swapper_output_account_info_before_swap = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )


    target_chain_id   = SOLANA_CHAIN_ID
    target_pool_id    = swap_interface_state
    target_program_id = swap_interface_program.program_id

    source_swap_id    = 0

    cross_chain_swap_result = await perform_cross_chain_swap(
        swapper_input_asset_balance,
        swapper_input_asset,
        swapper_input_asset_wallet,
        swapper.user_keypair,
        target_chain_id,
        target_pool_id,
        swapper_output_asset_index,
        swapper_output_asset_wallet,
        source_swap_id,  # Escrow nonce
        generic_payer,
        swap_pool_program,
        swap_pool_state,
        swap_pool_input_asset_wallet,
        swap_pool_authority,
        swap_interface_program,
        swap_interface_state,
        connection_state,
        polymerase_sender_program,
        polymerase_emulator_setup_master_keypair # Payer, can be anyone
    )

    sim_out_swap_units = catalyst_simulator.out_swap(
        swapper_input_asset,
        swapper_input_asset_balance,
        source_swap_id
    )

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Compare simulated out_swap units with the actual ones (from out_swap event)
    out_swap_units = u256_array_to_int(cross_chain_swap_result.out_swap_event.data.withdrawnPoolUnitsX64)
    assert sim_out_swap_units == out_swap_units



    # Skip Step 2: Do not perform cross-chain swap execute => receive => in_swap

    # Fetch payload
    data = await polymerase_sender_program.account["IbcData"].fetch(cross_chain_swap_result.ibc_data)

    # Compare the current state of the output asset destination wallet
    swapper_output_account_info_after_out_swap = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )

    # ! TODO add more tests

    before_balance = swapper_output_account_info_before_swap.amount
    after_balance  = swapper_output_account_info_after_out_swap.amount

    assert after_balance == before_balance - swapper_input_asset_balance



    # Step 3: Trigger ibc timeout on source chain (simulates polymer chain endpoint contract)
    swap_escrow = cross_chain_swap_result.swap_escrow
    ack_result = await swap_interface_program.rpc["on_timeout_packet"](
        data.payload,
        ctx=Context(
            accounts={
                "port_registration": port_registration,
                "interface_state_account": swap_interface_state,
                "swap_pool": swap_pool_state,
                "swap_pool_program": swap_pool_program.program_id,
                "swap_pool_authority": swap_pool_authority,
                "token_program": TOKEN_PROGRAM_ID,
                "system_program": SYS_PROGRAM_ID
            },
            remaining_accounts=[
                AccountMeta(swapper_output_asset,          is_signer=False, is_writable=False ), # output_asset_mint
                AccountMeta(swapper_output_asset_wallet,   is_signer=False, is_writable=True  ), # output_asset_wallet
                AccountMeta(swap_pool_output_asset_wallet, is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
                AccountMeta(swap_escrow,                   is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
                AccountMeta(generic_payer.public_key,      is_signer=False, is_writable=True  ), # swap_pool_output_asset_wallet
            ]
        )
    )
    await confirm_transaction(provider, ack_result)

    catalyst_simulator.out_swap_timeout(source_swap_id)

    await verify_catalyst_state(swap_pool_program, swap_pool_state, catalyst_simulator)

    # Compare the current state of the output asset destination wallet
    swapper_output_account_info_after_swap_timeout = await get_account_info(
        provider,
        swapper_output_asset,
        swapper_output_asset_wallet
    )

    before_balance         = swapper_output_account_info_before_swap.amount
    after_timeout_balance  = swapper_output_account_info_after_swap_timeout.amount

    assert before_balance == after_timeout_balance   # Timeout must return assets
