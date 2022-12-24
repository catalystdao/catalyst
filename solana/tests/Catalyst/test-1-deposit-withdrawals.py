from solana.keypair import Keypair
from solana.transaction import AccountMeta

from anchorpy import Context, Program, Provider
from conftest import UserWallet, create_mints, create_users
from utils.simulator_utils import create_and_verify_catalyst_simulator
from utils.account_utils import get_swap_pool_asset_wallet, get_swap_pool_authority
from utils.verify_utils import verify_catalyst_state, verify_deposit_event, verify_withdraw_event
from utils.transaction_utils import DEFAULT_TX_COMMITMENT, TxEventListener, confirm_transaction

from utils.swap_pool_utils import create_and_setup_swap_pool
from utils.token_utils import approve, create_token_account, fund_accounts, get_account_info
from spl.token.constants import TOKEN_PROGRAM_ID


async def test_deposits(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    swap_interface_program: Program,
    swap_interface_setup_master_keypair: Keypair,
    polymerase_register_program: Program,
    polymerase_sender_program: Program,
    mint_authority: Keypair,
    provider: Provider,
    generic_payer: Keypair
):
    swap_pool_state_keypair = Keypair()
    swap_pool_state         = swap_pool_state_keypair.public_key

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state,
    )[0]
    
    amplification   = None
    asset_count     = 3

    # Create new assets and mints on every run to isolate tests from each other
    assets         = await create_mints(provider, mint_authority, asset_count)
    assets_weights = [1 for _ in range(asset_count)]

    users          = await create_users(provider, assets, generic_payer, 2)

    # Initialize the pool
    creator                 = users[0]
    creator_keypair         = creator.user_keypair
    creator_asset_wallets   = creator.token_accounts
    init_asset_amounts      = [10*10**8, 100*10**8, 1000*10**6]
    
    await fund_accounts(
        provider                 = provider,
        mints                    = assets,
        mints_authority          = mint_authority,
        token_accounts           = creator_asset_wallets,
        balance                  = init_asset_amounts,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = init_asset_amounts,
        token_accounts_authority = creator_keypair
    )

    create_and_setup_swap_pool_result = await create_and_setup_swap_pool(
        swap_pool_program,
        swap_pool_setup_master_keypair,
        amplification,
        assets_weights,
        init_asset_amounts,
        assets,
        creator_asset_wallets,
        creator_keypair.public_key,
        swap_interface_program,
        swap_interface_setup_master_keypair,
        polymerase_register_program,
        polymerase_sender_program,
        swap_pool_state_keypair=swap_pool_state_keypair
    )

    swap_pool_state     = create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_state
    swap_pool_authority = create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_authority

    # Initialise and fund the catalyst simulator, match the deployed pool config and state with the simulator
    catalyst_simulator = await create_and_verify_catalyst_simulator(
        swap_pool_program,
        swap_pool_state,
        create_and_setup_swap_pool_result.initialize_swap_interface_result.swap_interface_state,
        amplification,
        assets,
        assets_weights,
        init_asset_amounts,
        creator_keypair.public_key
    )

    
    # Deposit assets
    depositor                  = users[1]
    depositor_keypair          = depositor.user_keypair
    depositor_asset_wallets    = depositor.token_accounts

    deposited_pool_tokens_base = 5000

    # Simulate deposit to get the required balance of each asset for depositing
    simulated_assets_deposit_amounts = catalyst_simulator.deposit(
        pool_tokens_amount=deposited_pool_tokens_base,
        user=depositor_keypair.public_key
    )
    asset_deposit_amounts = [simulated_assets_deposit_amounts[asset].value for asset in assets]
    
    await fund_accounts(
        provider                 = provider,
        mints                    = assets,
        mints_authority          = mint_authority,
        token_accounts           = depositor_asset_wallets,
        balance                  = asset_deposit_amounts,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = asset_deposit_amounts,
        token_accounts_authority = depositor_keypair
    )


    # Create pool token account for the depositor
    depositor_pool_token_wallet = await create_token_account(
        provider,
        create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_token_mint,
        depositor_keypair.public_key,
        generic_payer
    )

    # Group 'remaining accounts'
    remaining_accounts: list[AccountMeta] = []
    swap_pool_asset_wallets = []
    for i, mint in enumerate(assets):

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
            pubkey      = depositor_asset_wallets[i],
            is_signer   = False,
            is_writable = True
        ))
                    
    # Deposit
    async with TxEventListener("DepositEvent") as ev_listener:
        tx_result = await swap_pool_program.rpc["deposit"](
            deposited_pool_tokens_base,
            ctx=Context(
                accounts={
                    "swap_pool_state_account": swap_pool_state,
                    "depositor_pool_token_wallet": depositor_pool_token_wallet,
                    "swap_pool_token_mint": create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_token_mint,
                    "swap_pool_authority": swap_pool_authority,
                    "token_program": TOKEN_PROGRAM_ID
                },
                remaining_accounts=remaining_accounts,
                signers=[]
            )
        )
        await confirm_transaction(provider, tx_result)

        # Verify the deposit event
        deposit_event = (await ev_listener.get_events(swap_pool_program))[0]
        verify_deposit_event(
            deposit_event               = deposit_event,
            swap_pool                   = swap_pool_state,
            depositor_asset_wallets     = depositor_asset_wallets,
            depositor_pool_token_wallet = depositor_pool_token_wallet,
            withdrawn_pool_token_amount = deposited_pool_tokens_base
        )

        # Compare the pool tokens yielded by the deposit with the expected amount of the simulator
        assert [simulated_assets_deposit_amounts[asset].value for asset in assets] == deposit_event.data.depositedAssetAmounts

    await verify_catalyst_state(
        swap_pool_program,
        swap_pool_state,
        catalyst_simulator
    )

    # Verify that the depositor has received the pool tokens
    depositor_pool_token_account_info = await get_account_info(
        provider,
        create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_token_mint,
        depositor_pool_token_wallet
    )
    assert(depositor_pool_token_account_info.amount == catalyst_simulator.pool_tokens_distribution_i[depositor_keypair.public_key])

                    
    # Make sure the assets have been transferred from the depositor to the pool
    for asset_index in range(asset_count):
        # NOTE: this swap pool asset wallet verification is redundant, as the pool's asset balances have been checked 
        # with the verify_catalyst_state function
        swap_pool_asset_account_info = await get_account_info(
            provider,
            assets[asset_index],
            remaining_accounts[3*asset_index+1].pubkey,
            commitment=DEFAULT_TX_COMMITMENT
        )
        assert(swap_pool_asset_account_info.amount == init_asset_amounts[asset_index] + asset_deposit_amounts[asset_index])

        # Verify that all assets from the depositor have been transferred to the pool
        depositor_account_info = await get_account_info(
            provider,
            assets[asset_index],
            depositor_asset_wallets[asset_index],
            commitment=DEFAULT_TX_COMMITMENT
        )
        assert(depositor_account_info.amount == 0)


async def test_withdrawals(
    swap_pool_program: Program,
    swap_pool_setup_master_keypair: Keypair,
    swap_interface_program: Program,
    swap_interface_setup_master_keypair: Keypair,
    polymerase_register_program: Program,
    polymerase_sender_program: Program,
    mint_authority: Keypair,
    provider: Provider,
    generic_payer: Keypair
):
    swap_pool_state_keypair = Keypair()
    swap_pool_state         = swap_pool_state_keypair.public_key

    swap_pool_authority = get_swap_pool_authority(
        swap_pool_program.program_id,
        swap_pool_state,
    )[0]
    
    amplification   = None
    asset_count     = 3

    # Create new assets and mints on every run to isolate tests from each other
    assets         = await create_mints(provider, mint_authority, asset_count)
    assets_weights = [1 for _ in range(asset_count)]

    users          = await create_users(provider, assets, generic_payer, 2)

    # Initialize the pool
    creator                 = users[0]
    creator_keypair         = creator.user_keypair
    creator_asset_wallets   = creator.token_accounts
    init_asset_amounts      = [10*10**8, 100*10**8, 1000*10**6]
    
    await fund_accounts(
        provider                 = provider,
        mints                    = assets,
        mints_authority          = mint_authority,
        token_accounts           = creator_asset_wallets,
        balance                  = init_asset_amounts,
        delegate_authority       = swap_pool_authority,
        delegate_balance         = init_asset_amounts,
        token_accounts_authority = creator_keypair
    )

    create_and_setup_swap_pool_result = await create_and_setup_swap_pool(
        swap_pool_program,
        swap_pool_setup_master_keypair,
        amplification,
        assets_weights,
        init_asset_amounts,
        assets,
        creator_asset_wallets,
        creator_keypair.public_key,
        swap_interface_program,
        swap_interface_setup_master_keypair,
        polymerase_register_program,
        polymerase_sender_program,
        swap_pool_state_keypair=swap_pool_state_keypair
    )

    swap_pool_state     = create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_state
    swap_pool_authority = create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_authority

    # Initialise and fund the catalyst simulator, match the deployed pool config and state with the simulator
    catalyst_simulator = await create_and_verify_catalyst_simulator(
        swap_pool_program,
        swap_pool_state,
        create_and_setup_swap_pool_result.initialize_swap_interface_result.swap_interface_state,
        amplification,
        assets,
        assets_weights,
        init_asset_amounts,
        creator_keypair.public_key
    )

    # Withdraw assets
    withdrawer                   = creator
    withdrawer_keypair           = withdrawer.user_keypair
    withdrawer_pool_token_wallet = create_and_setup_swap_pool_result.initialize_swap_pool_state_result.depositor_pool_token_wallet_keypair.public_key
    withdrawer_asset_wallets     = withdrawer.token_accounts

    withdrawn_pool_tokens_base = 50000

    # Simulate the withdrawal
    simulated_assets_withdrawal_amounts = catalyst_simulator.withdraw(
        pool_tokens_amount=withdrawn_pool_tokens_base,
        user=withdrawer_keypair.public_key
    )
    asset_withdrawal_amounts = [simulated_assets_withdrawal_amounts[asset].value for asset in assets]

    # Give pool token burn allowance to the pool authority
    await approve(
        provider,
        withdrawer_pool_token_wallet,
        withdrawer_keypair,
        swap_pool_authority,
        withdrawn_pool_tokens_base
    )
    
    # Group 'remaining accounts'
    remaining_accounts: list[AccountMeta] = []
    swap_pool_asset_wallets = []
    for i, mint in enumerate(assets):

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
            pubkey      = withdrawer_asset_wallets[i],
            is_signer   = False,
            is_writable = True
        ))
    
    # Withdraw
    async with TxEventListener("WithdrawEvent") as ev_listener:
        tx_result = await swap_pool_program.rpc["withdraw"](
            withdrawn_pool_tokens_base,
            ctx=Context(
                accounts={
                    "swap_pool_state_account": swap_pool_state,
                    "withdrawer_pool_token_wallet": withdrawer_pool_token_wallet,
                    "swap_pool_token_mint": create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_token_mint,
                    "swap_pool_authority": swap_pool_authority,
                    "token_program": TOKEN_PROGRAM_ID
                },
                remaining_accounts=remaining_accounts
            )
        )
        await confirm_transaction(provider, tx_result)

        # Verify the withdraw event
        withdraw_event = (await ev_listener.get_events(swap_pool_program))[0]
        verify_withdraw_event(
            withdraw_event               = withdraw_event,
            swap_pool                    = swap_pool_state,
            withdrawer_asset_wallets     = withdrawer_asset_wallets,
            withdrawer_pool_token_wallet = withdrawer_pool_token_wallet,
            burnt_pool_token_amount      = withdrawn_pool_tokens_base
        )

        # Compare the withdrawn asset amounts with the expected ones of the simulator
        assert[simulated_assets_withdrawal_amounts[asset].value for asset in assets] == withdraw_event.data.withdrawnAssetAmounts

    await verify_catalyst_state(
        swap_pool_program,
        swap_pool_state,
        catalyst_simulator
    )

    # Verify that the withdrawer has sent the pool tokens
    withdrawer_pool_token_account_info = await get_account_info(
        provider,
        create_and_setup_swap_pool_result.initialize_swap_pool_state_result.swap_pool_token_mint,
        withdrawer_pool_token_wallet
    )
    assert withdrawer_pool_token_account_info.amount == catalyst_simulator.pool_tokens_distribution_i[withdrawer_keypair.public_key]
                    
    # Make sure the assets have been transferred from the pool to the withdrawer
    for asset_index in range(asset_count):
        # NOTE: this swap pool asset wallet verification is redundant, as the pool's asset balances have been checked 
        # with the verify_catalyst_state function
        swap_pool_asset_account_info = await get_account_info(
            provider,
            assets[asset_index],
            remaining_accounts[3*asset_index+1].pubkey,
            commitment=DEFAULT_TX_COMMITMENT
        )
        assert swap_pool_asset_account_info.amount == init_asset_amounts[asset_index] - asset_withdrawal_amounts[asset_index]

        # Verify that the asset has been received by the withdrawer
        withdrawer_account_info = await get_account_info(
            provider,
            assets[asset_index],
            withdrawer_asset_wallets[asset_index],
            commitment=DEFAULT_TX_COMMITMENT
        )
        assert(withdrawer_account_info.amount == asset_withdrawal_amounts[asset_index])

